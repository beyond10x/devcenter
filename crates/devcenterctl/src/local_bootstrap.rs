//! Private local deployment inputs. No upstream credentials are copied into this environment.

use super::{Target, capture, kube, load_owned, write_private};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Args;
use serde_json::{Value, json};
use std::{fs, path::Path, path::PathBuf, process::Command};

#[derive(Debug, Args)]
pub struct Prepare {
    #[command(flatten)]
    pub(super) target: Target,
    #[arg(long)]
    pub(super) baseline_values: PathBuf,
    #[arg(long)]
    pub(super) baseline_lock: PathBuf,
    #[arg(long)]
    pub(super) identity_image: String,
    #[arg(long)]
    pub(super) connectors_image: String,
    #[arg(long)]
    pub(super) provider_image: String,
}

pub fn prepare(args: &Prepare) -> Result<()> {
    let state = &args.target.state;
    let owned = load_owned(state)?;
    let app = format!("https://devcenter.localhost:{}", owned.https_port);
    let provider = "https://provider.devcenter.localhost".to_owned();
    let baseline: Value = serde_yaml::from_slice(&fs::read(&args.baseline_values)?)?;
    let mut values = local_values(&baseline, &app, &provider)?;
    let mut lock: toml::Value = toml::from_str(&fs::read_to_string(&args.baseline_lock)?)?;
    for (name, reference) in [
        ("identity", &args.identity_image),
        ("connectors", &args.connectors_image),
    ] {
        let current = &baseline["components"][name]["image"];
        let baseline_reference = format!(
            "{}@{}",
            current["repository"]
                .as_str()
                .context("baseline repository missing")?,
            current["digest"]
                .as_str()
                .context("baseline digest missing")?
        );
        if reference == &baseline_reference {
            continue;
        }
        let (repository, digest) = immutable(reference)?;
        values["components"][name]["image"] =
            json!({"repository":repository,"digest":digest,"pullPolicy":"IfNotPresent"});
        lock["images"][name] = toml::Value::try_from(
            json!({"reference":repository,"digest":digest,"version":"local-candidate"}),
        )?;
    }
    immutable(&args.provider_image)?;
    write_private(
        &state.join("provider-candidate.txt"),
        args.provider_image.as_bytes(),
    )?;
    create_keys(state)?;
    create_repository(state)?;
    write_private(
        &state.join("gitlab-fixture-client-secret"),
        b"local-gitlab-fixture-client-secret",
    )?;
    write_private(
        &state.join("fixture-provisioning.json"),
        &serde_json::to_vec(
            &json!([{"integration":"gitlab","credential":"oauth_client_secret","value_file":"gitlab-fixture-client-secret"}]),
        )?,
    )?;
    write_private(
        &state.join("values.local.yaml"),
        serde_yaml::to_string(&values)?.as_bytes(),
    )?;
    write_private(
        &state.join("deployment.local.lock.toml"),
        toml::to_string_pretty(&lock)?.as_bytes(),
    )?;
    let resources = resources(
        state,
        &baseline,
        &app,
        &provider,
        &args.provider_image,
        &owned.cluster,
    )?;
    let resource_file = state.join("bootstrap.json");
    write_private(
        &resource_file,
        &serde_json::to_vec(&json!({"apiVersion":"v1","kind":"List","items":resources}))?,
    )?;
    capture(
        state,
        "bootstrap-apply",
        kube(state).args(["apply", "-f"]).arg(&resource_file),
    )?;
    configure_dns(state, owned.https_port)?;
    configure_token_review(state)?;
    println!("prepared local composition at {app}; model provider: deterministic fixture");
    Ok(())
}

fn immutable(image: &str) -> Result<(&str, &str)> {
    let (repository, digest) = image
        .rsplit_once('@')
        .context("candidate image needs an immutable digest")?;
    ensure!(
        repository.starts_with("k3d-devcenter-")
            && digest.starts_with("sha256:")
            && digest.len() == 71
            && digest[7..].bytes().all(|b| b.is_ascii_hexdigit()),
        "candidate image must be pinned in the local registry"
    );
    Ok((repository, digest))
}

#[allow(clippy::too_many_lines)] // The deployment overlay is kept together for review.
fn local_values(baseline: &Value, app: &str, provider: &str) -> Result<Value> {
    let mut values = baseline.clone();
    values["global"] = json!({"tenantId":"local-acceptance","publicOrigin":app,"imagePullSecrets":["github-container-registry"],"podLabels":{}});
    values["devcenter"]["identity"]["redirectUri"] = json!(format!("{app}/auth/sso/callback"));
    values["devcenter"]["identity"]["providers"] =
        json!([{"id":"default","display_name":"Local test provider"}]);
    values["ingress"]["className"] = json!("traefik");
    values["ingress"]["host"] = json!("devcenter.localhost");
    values["ingress"]["annotations"] = json!({});
    values["ingress"]["tls"]["secretName"] = json!("devcenter-local-tls");
    values["networkPolicy"]["ingressNamespaceSelector"] =
        json!({"kubernetes.io/metadata.name":"kube-system"});
    values["networkPolicy"]["ingressPodSelector"] = json!({"app.kubernetes.io/name":"traefik"});
    values["networkPolicy"]["allowExternalHttps"] = json!(false);
    values["networkPolicy"]["extraEgress"] = json!([
        {"to":[{"namespaceSelector":{"matchLabels":{"kubernetes.io/metadata.name":"kube-system"}},"podSelector":{"matchLabels":{"app.kubernetes.io/name":"traefik"}}}],"ports":[{"protocol":"TCP","port":8443}]}
    ]);
    values["connectorsKubernetesAccess"] = json!({"enabled":false});
    let components = values["components"]
        .as_object_mut()
        .context("baseline components missing")?;
    for component in components.values_mut() {
        if let Some(persistence) = component.get_mut("persistence") {
            persistence
                .as_object_mut()
                .context("persistence object")?
                .remove("claimName");
            persistence["storageClass"] = json!("local-path");
        }
    }
    let identity = &mut components["identity"];
    let mut audiences: Value = serde_json::from_str(
        identity["env"]["IDENTITY_AUDIENCE_REGISTRY_JSON"]
            .as_str()
            .context("Identity audience registry missing")?,
    )?;
    for access in audiences["access"]
        .as_array_mut()
        .context("access audiences missing")?
    {
        if access["audience"]
            .as_str()
            .is_some_and(|a| a.ends_with("/mcp"))
        {
            access["audience"] = json!(format!("{app}/mcp"));
        }
    }
    identity["env"] = json!({
        "IDENTITY_LISTEN":"0.0.0.0:8080","IDENTITY_PUBLIC_ORIGIN":app,"IDENTITY_TENANT_ID":"local-acceptance",
        "IDENTITY_CLI_CLIENT_ID":"devcenter-cli","IDENTITY_UPSTREAM_ISSUER":format!("{provider}/oidc"),
        "IDENTITY_WEB_CLIENTS_JSON":serde_json::to_string(&json!([{"clientId":"devcenter-web","redirectUri":format!("{app}/auth/sso/callback")}]))?,
        "IDENTITY_AUDIENCE_REGISTRY_JSON":serde_json::to_string(&audiences)?,
        "IDENTITY_DEFAULT_TENANT_GROUPS_JSON":"[{\"tenantId\":\"local-acceptance\",\"groups\":[\"engineer\"]}]",
        "IDENTITY_STATIC_GROUP_MEMBERSHIPS_JSON":"[{\"tenantId\":\"local-acceptance\",\"email\":\"engineer@example.test\",\"groups\":[\"operator\"]}]",
        "IDENTITY_DATABASE_PATH":"/var/lib/identity/private/identity.sqlite3","IDENTITY_UPSTREAM_CA_BUNDLE":"/etc/local-trust/ca.crt"
    });
    trust_mount(identity);
    let connectors = &mut components["connectors"];
    let original: toml::Value = toml::from_str(
        connectors["configFiles"]["hosted.toml"]
            .as_str()
            .context("Connector config missing")?,
    )?;
    let mut config = original;
    config
        .as_table_mut()
        .context("Connector config table")?
        .remove("slack");
    config["sip"] = toml::Value::try_from(json!({"enabled":false}))?;
    config["kubernetes"] = toml::Value::try_from(json!({"enabled":false,"namespace_access":[]}))?;
    config["tenant_id"] = toml::Value::String("local-acceptance".into());
    config["module_tenant_ids"] =
        toml::Value::Array(vec![toml::Value::String("local-acceptance".into())]);
    config["identity"]["origin"] = toml::Value::String(app.into());
    config["catalog"]["public_origin"] = toml::Value::String(format!("{app}/api/connectors/v1"));
    config["gitlab"]["origin"] = toml::Value::String(provider.into());
    config["gitlab"]["public_origin"] = toml::Value::String(format!("{app}/api/connectors/v1"));
    config["gitlab"]["oauth_client_id"] = toml::Value::String("local-gitlab".into());
    config["gitlab"]["oauth_redirect_uri"] =
        toml::Value::String(format!("{app}/api/connectors/v1/oauth/gitlab/callback"));
    connectors["configFiles"]["hosted.toml"] = json!(toml::to_string_pretty(&config)?);
    connectors["env"]["SSL_CERT_FILE"] = json!("/etc/local-trust/ca.crt");
    connectors["envFrom"] = json!([]);
    trust_mount(connectors);
    set_argument(
        components["agent-platform"]["args"]
            .as_array_mut()
            .context("Agent arguments missing")?,
        "--model-endpoint-base",
        "http://devcenter-local-provider.devcenter.svc.cluster.local:8080/v1",
    )?;
    let aep_args = components["aep-service"]["args"]
        .as_array_mut()
        .context("AEP arguments missing")?;
    if let Some(index) = aep_args.iter().position(|s| s == "--realm") {
        *aep_args
            .get_mut(index + 1)
            .context("AEP realm value missing")? = json!("local-acceptance");
    }
    components["workspace"]["env"]["WORKSPACE_AEP_REALM"] = json!("local-acceptance");
    values["secrets"]["tokenReviewRbac"]["create"] = json!(true);
    values["secrets"]["postgresql"]["storageClass"] = json!("local-path");
    values["secrets"]["postgresql"]["size"] = json!("1Gi");
    values["substrate"]["nodeSelector"] =
        json!({"kubernetes.io/arch":"amd64","kubernetes.io/os":"linux"});
    values["substrate"]["tls"]["existingSecret"] = json!("devcenter-local-tls");
    values["substrate"]["hostedIdentity"] =
        json!({"origin":app,"caBundle":{"existingSecret":"devcenter-local-trust","key":"ca.crt"}});
    values["substrate"]["client"] = json!({"serverIdentity":"devcenter-substrate.devcenter.svc.cluster.local","trustRoots":{"existingSecret":"devcenter-local-trust","key":"ca.crt"}});
    values["substrate"]["storage"] = json!({"className":"local-path","size":"1Gi"});
    values["connectorsGitFetch"]["tls"]["existingSecret"] = json!("devcenter-local-tls");
    values["connectorsGitFetch"]["trustRoots"]["existingSecret"] = json!("devcenter-local-trust");
    Ok(values)
}

fn set_argument(args: &mut Vec<Value>, flag: &str, value: &str) -> Result<()> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == flag {
            ensure!(
                args.get(index + 1)
                    .and_then(Value::as_str)
                    .is_some_and(|v| !v.starts_with("--")),
                "argument value missing for {flag}"
            );
            args.drain(index..index + 2);
        } else if args[index]
            .as_str()
            .is_some_and(|arg| arg.starts_with(&format!("{flag}=")))
        {
            args.remove(index);
        } else {
            index += 1;
        }
    }
    args.extend([json!(flag), json!(value)]);
    Ok(())
}

fn trust_mount(component: &mut Value) {
    component["volumes"] =
        json!([{"name":"local-trust","secret":{"secretName":"devcenter-local-trust"}}]);
    component["volumeMounts"] =
        json!([{"name":"local-trust","mountPath":"/etc/local-trust","readOnly":true}]);
}

fn random_file(state: &Path, name: &str) -> Result<String> {
    let path = state.join(name);
    if !path.exists() {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| anyhow::anyhow!("random source unavailable"))?;
        write_private(&path, STANDARD.encode(bytes).as_bytes())?;
    }
    super::private_file(&path)?;
    Ok(fs::read_to_string(path)?)
}

fn create_keys(state: &Path) -> Result<()> {
    if !state.join("ca.crt").exists() {
        capture(
            state,
            "ca-create",
            Command::new("openssl").current_dir(state).args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-noenc",
                "-keyout",
                "ca.key",
                "-out",
                "ca.crt",
                "-days",
                "7",
                "-subj",
                "/CN=Devcenter local acceptance CA",
                "-addext",
                "basicConstraints=critical,CA:TRUE",
                "-addext",
                "keyUsage=critical,keyCertSign,cRLSign",
            ]),
        )?;
        capture(
            state,
            "tls-request",
            Command::new("openssl").current_dir(state).args([
                "req",
                "-new",
                "-newkey",
                "rsa:2048",
                "-noenc",
                "-keyout",
                "tls.key",
                "-out",
                "tls.csr",
                "-subj",
                "/CN=devcenter.localhost",
            ]),
        )?;
        write_private(&state.join("tls.ext"), b"basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:devcenter.localhost,DNS:provider.devcenter.localhost,DNS:devcenter-substrate.devcenter.svc.cluster.local,DNS:devcenter-connectors.devcenter.svc.cluster.local\n")?;
        capture(
            state,
            "tls-sign",
            Command::new("openssl").current_dir(state).args([
                "x509",
                "-req",
                "-in",
                "tls.csr",
                "-CA",
                "ca.crt",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-out",
                "tls.crt",
                "-days",
                "7",
                "-extfile",
                "tls.ext",
            ]),
        )?;
    }
    if !state.join("oidc-signing.key").exists() {
        capture(
            state,
            "oidc-key",
            Command::new("openssl").current_dir(state).args([
                "genrsa",
                "-out",
                "oidc-signing.key",
                "2048",
            ]),
        )?;
    }
    random_file(state, "oidc-client-secret")?;
    Ok(())
}

fn create_repository(state: &Path) -> Result<()> {
    let source = state.join("fixture-source");
    let repository = state.join("repositories/fixture/workspace.git");
    if !repository.exists() {
        fs::create_dir_all(&source)?;
        fs::create_dir_all(repository.parent().context("repository parent")?)?;
        capture(
            state,
            "fixture-git-init",
            Command::new("git")
                .arg("init")
                .args(["--initial-branch=main"])
                .arg(&source),
        )?;
        let mut content = String::new();
        for n in 1..=80 {
            use std::fmt::Write;
            writeln!(content, "Local acceptance line {n:02}")?;
        }
        write_private(&source.join("README.md"), content.as_bytes())?;
        capture(
            state,
            "fixture-git-add",
            Command::new("git")
                .current_dir(&source)
                .args(["add", "README.md"]),
        )?;
        capture(
            state,
            "fixture-git-commit",
            Command::new("git").current_dir(&source).args([
                "-c",
                "user.name=Local acceptance fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "-m",
                "Local acceptance fixture",
            ]),
        )?;
        capture(
            state,
            "fixture-git-clone",
            Command::new("git")
                .args(["clone", "--bare"])
                .arg(&source)
                .arg(&repository),
        )?;
    }
    capture(
        state,
        "fixture-git-archive",
        Command::new("tar")
            .arg("-czf")
            .arg(state.join("repositories.tar.gz"))
            .arg("-C")
            .arg(state)
            .arg("repositories"),
    )?;
    Ok(())
}

fn secret(name: &str, data: Value) -> Value {
    let mut resource = json!({"apiVersion":"v1","kind":"Secret","metadata":{"name":name,"namespace":"devcenter"},"type":"Opaque","stringData":null});
    resource["stringData"] = data;
    resource
}

fn resources(
    state: &Path,
    baseline: &Value,
    app: &str,
    provider: &str,
    provider_image: &str,
    cluster: &str,
) -> Result<Vec<Value>> {
    let secrets_password = random_file(state, "secrets-database-password")?;
    let app_password = random_file(state, "app-database-password")?;
    let encoded_app =
        url::form_urlencoded::byte_serialize(app_password.as_bytes()).collect::<String>();
    let encoded_secrets =
        url::form_urlencoded::byte_serialize(secrets_password.as_bytes()).collect::<String>();
    let key = random_file(state, "secrets-key")?;
    let mut docs = vec![
        json!({"apiVersion":"v1","kind":"Namespace","metadata":{"name":"devcenter"}}),
        secret(
            "devcenter-local-trust",
            json!({"ca.crt":fs::read_to_string(state.join("ca.crt"))?}),
        ),
        secret(
            "devcenter-local-tls",
            json!({"tls.crt":fs::read_to_string(state.join("tls.crt"))?,"tls.key":fs::read_to_string(state.join("tls.key"))?}),
        ),
        secret(
            "devcenter-identity-upstream",
            json!({"IDENTITY_UPSTREAM_CLIENT_ID":"local-identity","IDENTITY_UPSTREAM_CLIENT_SECRET":fs::read_to_string(state.join("oidc-client-secret"))?}),
        ),
        secret(
            "devcenter-secrets-database",
            json!({"password":secrets_password,"database-url":format!("postgres://secrets:{encoded_secrets}@devcenter-secrets-postgresql.devcenter.svc.cluster.local:5432/secrets")}),
        ),
        secret(
            "devcenter-database",
            json!({"password":app_password,"database-url":format!("postgres://devcenter:{encoded_app}@devcenter-local-postgresql.devcenter.svc.cluster.local:5432/devcenter")}),
        ),
        secret(
            "devcenter-secrets-keyring",
            json!({"keyring.json":serde_json::to_string(&json!({"active":"v1","keys":{"v1":key}}))?}),
        ),
        json!({"apiVersion":"v1","kind":"Secret","metadata":{"name":"devcenter-local-provider","namespace":"devcenter"},"data":{"oidc-signing.key":STANDARD.encode(fs::read(state.join("oidc-signing.key"))?),"oidc-client-secret":STANDARD.encode(fs::read(state.join("oidc-client-secret"))?),"repositories.tar.gz":STANDARD.encode(fs::read(state.join("repositories.tar.gz"))?)}}),
        json!({"apiVersion":"v1","kind":"PersistentVolume","metadata":{"name":"devcenter-local-workspaces"},"spec":{"capacity":{"storage":"4Gi"},"volumeMode":"Filesystem","accessModes":["ReadWriteOnce"],"persistentVolumeReclaimPolicy":"Retain","storageClassName":"devcenter-local-quota","local":{"path":"/var/lib/devcenter-local/workspaces"},"nodeAffinity":{"required":{"nodeSelectorTerms":[{"matchExpressions":[{"key":"kubernetes.io/hostname","operator":"In","values":[format!("k3d-{cluster}-server-0")]}]}]}}}}),
        json!({"apiVersion":"v1","kind":"PersistentVolumeClaim","metadata":{"name":"devcenter-substrate-workspaces","namespace":"devcenter"},"spec":{"accessModes":["ReadWriteOnce"],"storageClassName":"devcenter-local-quota","volumeName":"devcenter-local-workspaces","resources":{"requests":{"storage":"4Gi"}}}}),
    ];
    let labels = json!({"app.kubernetes.io/instance":"devcenter","app.kubernetes.io/component":"local-provider"});
    docs.push(json!({"apiVersion":"apps/v1","kind":"Deployment","metadata":{"name":"devcenter-local-provider","namespace":"devcenter"},"spec":{"replicas":1,"selector":{"matchLabels":labels},"template":{"metadata":{"labels":labels},"spec":{"securityContext":{"runAsUser":65532,"runAsGroup":65532,"fsGroup":65532,"runAsNonRoot":true,"seccompProfile":{"type":"RuntimeDefault"}},"initContainers":[{"name":"fixture-state","image":provider_image,"command":["/bin/sh","-c","install -m 600 /input/oidc-signing.key /input/oidc-client-secret /state/ && tar xzf /input/repositories.tar.gz -C /state"],"securityContext":{"allowPrivilegeEscalation":false,"capabilities":{"drop":["ALL"]}},"volumeMounts":[{"name":"input","mountPath":"/input","readOnly":true},{"name":"state","mountPath":"/state"}]}],"containers":[{"name":"provider","image":provider_image,"args":["--origin",provider,"--app-origin",app,"--state","/state"],"env":[{"name":"LOCAL_ACCEPTANCE_FIXTURE","value":"1"}],"ports":[{"name":"http","containerPort":8080}],"readinessProbe":{"httpGet":{"path":"/readyz","port":"http"}},"securityContext":{"allowPrivilegeEscalation":false,"readOnlyRootFilesystem":true,"capabilities":{"drop":["ALL"]}},"volumeMounts":[{"name":"state","mountPath":"/state"}]}],"volumes":[{"name":"input","secret":{"secretName":"devcenter-local-provider"}},{"name":"state","emptyDir":{}}]}}}}));
    docs.push(json!({"apiVersion":"v1","kind":"Service","metadata":{"name":"devcenter-local-provider","namespace":"devcenter"},"spec":{"selector":labels,"ports":[{"port":8080,"targetPort":"http"}]}}));
    docs.push(json!({"apiVersion":"networking.k8s.io/v1","kind":"Ingress","metadata":{"name":"devcenter-local-provider","namespace":"devcenter"},"spec":{"ingressClassName":"traefik","tls":[{"hosts":["provider.devcenter.localhost"],"secretName":"devcenter-local-tls"}],"rules":[{"host":"provider.devcenter.localhost","http":{"paths":[{"path":"/","pathType":"Prefix","backend":{"service":{"name":"devcenter-local-provider","port":{"number":8080}}}}]}}]}}));
    let postgres_image = &baseline["secrets"]["postgresql"]["image"];
    let postgres = format!(
        "{}@{}",
        postgres_image["repository"]
            .as_str()
            .context("postgres repository")?,
        postgres_image["digest"]
            .as_str()
            .context("postgres digest")?
    );
    let db_labels = json!({"app.kubernetes.io/instance":"devcenter","app.kubernetes.io/component":"local-postgresql"});
    docs.push(json!({"apiVersion":"v1","kind":"PersistentVolumeClaim","metadata":{"name":"devcenter-local-postgresql","namespace":"devcenter"},"spec":{"accessModes":["ReadWriteOnce"],"storageClassName":"local-path","resources":{"requests":{"storage":"1Gi"}}}}));
    docs.push(json!({"apiVersion":"apps/v1","kind":"Deployment","metadata":{"name":"devcenter-local-postgresql","namespace":"devcenter"},"spec":{"strategy":{"type":"Recreate"},"replicas":1,"selector":{"matchLabels":db_labels},"template":{"metadata":{"labels":db_labels},"spec":{"securityContext":{"runAsUser":999,"runAsGroup":999,"fsGroup":999,"seccompProfile":{"type":"RuntimeDefault"}},"containers":[{"name":"postgresql","image":postgres,"env":[{"name":"POSTGRES_DB","value":"devcenter"},{"name":"POSTGRES_USER","value":"devcenter"},{"name":"POSTGRES_PASSWORD","valueFrom":{"secretKeyRef":{"name":"devcenter-database","key":"password"}}},{"name":"PGDATA","value":"/var/lib/postgresql/data/pgdata"}],"ports":[{"name":"postgresql","containerPort":5432}],"readinessProbe":{"exec":{"command":["pg_isready","-U","devcenter","-d","devcenter"]}},"securityContext":{"allowPrivilegeEscalation":false,"capabilities":{"drop":["ALL"]}},"volumeMounts":[{"name":"data","mountPath":"/var/lib/postgresql/data"}]}],"volumes":[{"name":"data","persistentVolumeClaim":{"claimName":"devcenter-local-postgresql"}}]}}}}));
    docs.push(json!({"apiVersion":"v1","kind":"Service","metadata":{"name":"devcenter-local-postgresql","namespace":"devcenter"},"spec":{"selector":db_labels,"ports":[{"port":5432,"targetPort":"postgresql"}]}}));
    Ok(docs)
}

fn configure_dns(state: &Path, port: u16) -> Result<()> {
    let service: Value = serde_json::from_slice(&capture(
        state,
        "ingress-service",
        kube(state).args([
            "-n",
            "kube-system",
            "get",
            "service",
            "traefik",
            "-o",
            "json",
        ]),
    )?)?;
    let address = service["spec"]["clusterIP"]
        .as_str()
        .context("Traefik ClusterIP missing")?;
    let mut ports = service["spec"]["ports"]
        .as_array()
        .context("ingress ports missing")?
        .clone();
    if !ports.iter().any(|p| p["port"] == port) {
        ports.push(
            json!({"name":"local-https","port":port,"targetPort":"websecure","protocol":"TCP"}),
        );
    }
    let patch = state.join("ingress-service.patch.json");
    write_private(
        &patch,
        &serde_json::to_vec(&json!({"spec":{"ports":ports}}))?,
    )?;
    capture(
        state,
        "ingress-service-patch",
        kube(state)
            .args([
                "-n",
                "kube-system",
                "patch",
                "service",
                "traefik",
                "--type=merge",
                "--patch-file",
            ])
            .arg(patch),
    )?;
    let config: Value = serde_json::from_slice(&capture(
        state,
        "dns-config",
        kube(state).args([
            "-n",
            "kube-system",
            "get",
            "configmap",
            "coredns",
            "-o",
            "json",
        ]),
    )?)?;
    let hosts = config["data"]["NodeHosts"]
        .as_str()
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.contains("devcenter.localhost"))
        .collect::<Vec<_>>()
        .join("\n");
    let patch = state.join("dns.patch.json");
    write_private(
        &patch,
        &serde_json::to_vec(
            &json!({"data":{"NodeHosts":format!("{hosts}\n{address} devcenter.localhost provider.devcenter.localhost\n")}}),
        )?,
    )?;
    capture(
        state,
        "dns-patch",
        kube(state)
            .args([
                "-n",
                "kube-system",
                "patch",
                "configmap",
                "coredns",
                "--type=merge",
                "--patch-file",
            ])
            .arg(patch),
    )?;
    Ok(())
}

fn configure_token_review(state: &Path) -> Result<()> {
    let endpoints: Value = serde_json::from_slice(&capture(
        state,
        "api-endpoints",
        kube(state).args([
            "-n",
            "default",
            "get",
            "endpoints",
            "kubernetes",
            "-o",
            "json",
        ]),
    )?)?;
    let mut egress = Vec::new();
    for subset in endpoints["subsets"]
        .as_array()
        .context("API endpoint subsets missing")?
    {
        for address in subset["addresses"]
            .as_array()
            .context("API addresses missing")?
        {
            let ip: std::net::Ipv4Addr =
                address["ip"].as_str().context("API IP missing")?.parse()?;
            for port in subset["ports"].as_array().context("API ports missing")? {
                egress.push(json!({"to":[{"ipBlock":{"cidr":format!("{ip}/32")}}],"ports":[{"protocol":"TCP","port":port["port"]}]}));
            }
        }
    }
    ensure!(!egress.is_empty(), "API endpoints absent");
    let file = state.join("secrets-api-egress.json");
    write_private(
        &file,
        &serde_json::to_vec(
            &json!({"apiVersion":"networking.k8s.io/v1","kind":"NetworkPolicy","metadata":{"name":"devcenter-local-secrets-api","namespace":"devcenter"},"spec":{"podSelector":{"matchLabels":{"app.kubernetes.io/instance":"devcenter","app.kubernetes.io/component":"secrets"}},"policyTypes":["Egress"],"egress":egress}}),
        )?,
    )?;
    capture(
        state,
        "secrets-api-egress",
        kube(state).args(["apply", "-f"]).arg(file),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_local_overlay_replaces_endpoint_without_duplicate_flags() {
        let mut args = vec![json!("serve"), json!("--model-endpoint-base=old")];
        set_argument(
            &mut args,
            "--model-endpoint-base",
            "http://provider.localhost/v1",
        )
        .unwrap();
        let first = args.clone();
        set_argument(
            &mut args,
            "--model-endpoint-base",
            "http://provider.localhost/v1",
        )
        .unwrap();
        assert_eq!(args, first);
        assert_eq!(args.len(), 3);
    }
}
