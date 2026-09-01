import { readFile } from "node:fs/promises";
import openapiTS, { astToString, COMMENT_HEADER } from "openapi-typescript";

const contract = new URL("../../openapi.json", import.meta.url);
const checkedInTypes = new URL("../src/api/schema.gen.ts", import.meta.url);
const expected = COMMENT_HEADER + astToString(await openapiTS(contract));
const actual = await readFile(checkedInTypes, "utf8");

if (actual !== expected) {
  console.error(
    "Generated API types are stale. Run `pnpm --dir frontend generate:types` and commit the result.",
  );
  process.exitCode = 1;
}
