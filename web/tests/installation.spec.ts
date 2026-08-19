import { test, expect, chromium, type Page } from "@playwright/test";
import { spawn } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { createHash, randomUUID, X509Certificate } from "node:crypto";

const config = process.env.RECOLLECT_TEST_INSTALLATION_CONFIG;
const saved = config ? JSON.parse(readFileSync(config, "utf8")) : undefined;
test.skip(!saved, "Select the private result of test-installation-runtime.py");

test("installed desktop owner/member isolation, evidence and native HTTPS pairing", async () => {
  test.setTimeout(120_000);
  expect(saved.instance).toMatch(/^proof-/);
  const shared = saved.origin.startsWith("https:");
  const args: string[] = [];
  if (shared) {
    // This disposable browser's exact fixture key exception is not TLS proof.
    // Python and native clients separately require normal CA/hostname validation.
    const control = await chromium.launch({ channel: "chrome" });
    try {
      const page = await control.newPage();
      await expect(page.goto(saved.origin)).rejects.toThrow(
        /ERR_CERT_AUTHORITY_INVALID/,
      );
    } finally {
      await control.close();
    }
    const certificate = new X509Certificate(
      readFileSync(resolve(dirname(saved.ca), "server.pem")),
    );
    const digest = createHash("sha256")
      .update(certificate.publicKey.export({ type: "spki", format: "der" }))
      .digest("base64");
    args.push(`--ignore-certificate-errors-spki-list=${digest}`);
  }
  const browser = await chromium.launch({ channel: "chrome", args });
  const errors: string[] = [];
  const ownerContext = await browser.newContext({
    viewport: { width: 1440, height: 960 },
  });
  const memberContext = await browser.newContext({
    viewport: { width: 1440, height: 960 },
  });
  const owner = await ownerContext.newPage();
  const member = await memberContext.newPage();
  for (const page of [owner, member]) {
    page.setDefaultTimeout(10_000);
    page.on("pageerror", (error) => errors.push(error.message));
  }
  async function login(
    page: Page,
    account: { username: string; password: string },
  ) {
    await page.goto(saved.origin);
    await page.getByLabel(/^Username/).fill(account.username);
    await page.getByLabel(/^Password/).fill(account.password);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "Your Brains", exact: true }),
    ).toBeVisible();
  }
  try {
    await login(owner, saved.owner);
    await expect(
      owner.getByText(saved.member_brain.name, { exact: true }),
    ).toHaveCount(0);
    await owner
      .getByRole("button", { name: "Runtime diagnostics", exact: true })
      .click();
    await expect(
      owner.getByRole("dialog").getByText("Requests started", { exact: true }),
    ).toBeVisible();
    await owner.screenshot({
      path: resolve(dirname(config!), "owner-desktop.png"),
      animations: "disabled",
    });
    await owner.keyboard.press("Escape");
    await owner.goto(`${saved.origin}/brains/${saved.brain.id}`);
    await expect(
      owner.getByRole("heading", { name: saved.brain.name, exact: true }),
    ).toBeVisible();

    await login(member, saved.member);
    await expect(
      member.getByRole("button", { name: "Runtime diagnostics", exact: true }),
    ).toHaveCount(0);
    await expect(
      member.getByText(saved.member_brain.name, { exact: true }),
    ).toBeVisible();
    await member.goto(`${saved.origin}/brains/${saved.brain.id}`);
    await expect(
      member.getByRole("heading", { name: saved.brain.name, exact: true }),
    ).toBeVisible();
    await expect(
      member.getByRole("button", { name: "Import source", exact: true }),
    ).toHaveCount(0);
    await member
      .getByRole("button", { name: new RegExp(saved.brain.name) })
      .first()
      .click();
    await expect(member.getByTestId("source-content")).toContainText(
      saved.brain.name,
    );
    await member.screenshot({
      path: resolve(dirname(config!), "member-desktop.png"),
      animations: "disabled",
    });
    if (shared) await nativePairing(owner);
    expect(errors).toEqual([]);
  } finally {
    await browser.close();
  }
});

async function nativePairing(page: Page) {
  const root = resolve("..");
  const proof = resolve(dirname(config!));
  const name = `recollect-installation-native-${randomUUID().slice(0, 12)}`;
  const envFile = resolve(proof, `${name}.env`);
  writeFileSync(
    envFile,
    [
      "RECOLLECT_URL=https://localhost:8443",
      "RECOLLECT_CA_FILE=/fixture-ca.pem",
      `RECOLLECT_DEVICE_PROFILE=installation-proof-${randomUUID()}`,
      `RECOLLECT_FIXTURE_KEYRING_PASSWORD=${randomUUID()}`,
      `RECOLLECT_TEST_BRAIN=${saved.brain.id}`,
      `RECOLLECT_TEST_MARKER=${saved.brain.name}`,
      "CARGO_INCREMENTAL=0",
      "CARGO_PROFILE_DEV_DEBUG=0",
      "CARGO_PROFILE_TEST_DEBUG=0",
      "HOME=/tmp/installation-home",
    ].join("\n") + "\n",
    { mode: 0o600, flag: "wx" },
  );
  mkdirSync(resolve(root, ".cache/mcp-hosts/work"), { recursive: true });
  const environment = Object.fromEntries(
    Object.entries(process.env).filter(([key]) =>
      [
        "PATH",
        "HOME",
        "USER",
        "TMPDIR",
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
      ].includes(key),
    ),
  );
  const docker = resolve(root, "scripts/docker.sh");
  const child = spawn(
    docker,
    [
      "run",
      "--rm",
      "--init",
      "--name",
      name,
      "--label",
      "io.recollect.owner=installation-native-proof",
      "--network",
      `container:recollect-install-${saved.instance}-proxy-1`,
      "--memory",
      "1g",
      "--cpus",
      "2",
      "--pids-limit",
      "256",
      "--env-file",
      envFile,
      "-v",
      `${root}/Cargo.toml:/workspace/Cargo.toml:ro`,
      "-v",
      `${root}/Cargo.lock:/workspace/Cargo.lock:ro`,
      "-v",
      `${root}/crates:/workspace/crates:ro`,
      "-v",
      `${root}/.cache/mcp-hosts/target:/workspace/target`,
      "-v",
      `${root}/.cache/capture-hosts/cargo:/usr/local/cargo/registry`,
      "-v",
      `${saved.ca}:/fixture-ca.pem:ro`,
      "recollect-mcp-hosts:local",
      "bash",
      "-c",
      'mkdir -p "$HOME" /tmp/installation-workspace; exec dbus-run-session -- recollect-test-session bash -c \'set -e; /workspace/target/debug/recollect-agent pair "Installed HTTPS companion"; cargo test --locked --offline -p recollect-mcp-runtime --test installed_native -- --ignored --nocapture\'',
    ],
    { env: environment, stdio: ["ignore", "pipe", "pipe"] },
  );
  let output = "";
  child.stdout.on("data", (value) => {
    output += value.toString();
  });
  child.stderr.on("data", (value) => {
    output += value.toString();
  });
  const finished = new Promise<number | null>((done, reject) => {
    child.once("exit", done);
    child.once("error", reject);
  });
  try {
    await expect
      .poll(
        () => /Open (https:\/\/\S+)/.test(output) || child.exitCode !== null,
        { timeout: 20_000 },
      )
      .toBe(true);
    const link = output.match(/Open (https:\/\/\S+)/)?.[1];
    expect(
      link,
      "Native pairing must offer its real verification link",
    ).toBeTruthy();
    await page.goto(link!);
    await expect(
      page.getByRole("heading", { name: "Approve a companion", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText(new URL(link!).searchParams.get("code")!, { exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Approve device", exact: true })
      .click();
    await expect(
      page.getByText("Your companion is connected.", { exact: true }),
    ).toBeVisible({ timeout: 20_000 });
    const result = await Promise.race([
      finished,
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error("Native fixture deadline")), 60_000),
      ),
    ]);
    expect(result, "Inspect the private native fixture log").toBe(0);
    expect(output).toContain("1 passed");
    await page.screenshot({
      path: resolve(proof, "native-pairing-desktop.png"),
      animations: "disabled",
    });
  } finally {
    writeFileSync(resolve(proof, "native.log"), output, { mode: 0o600 });
    if (child.exitCode === null) {
      const cleanup = spawn(docker, ["stop", "--time", "10", name], {
        env: environment,
        stdio: "ignore",
      });
      await new Promise((done) => cleanup.once("exit", done));
    }
  }
}
