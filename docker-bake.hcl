group "default" {
  targets = ["chart", "connectors", "deployment-cli", "server"]
}

target "chart" {
  context = "."
  dockerfile = "Dockerfile.ess"
  target = "chart_artifact"
  platforms = ["linux/amd64", "linux/arm64"]
  output = ["type=local,dest=out/chart"]
}

target "connectors" {
  context = "."
  dockerfile = "Dockerfile.ess"
  target = "connectors_image"
  platforms = ["linux/amd64", "linux/arm64"]
}

target "deployment-cli" {
  context = "."
  dockerfile = "Dockerfile.ess"
  target = "ctl_image"
  platforms = ["linux/amd64", "linux/arm64"]
}

target "server" {
  context = "."
  dockerfile = "Dockerfile.ess"
  target = "server_image"
  platforms = ["linux/amd64", "linux/arm64"]
}

