import { test, expect, type Page } from "@playwright/test";
import { randomUUID } from "node:crypto";

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel(/^Username/).fill(process.env.RECOLLECT_OWNER_USERNAME!);
  await page.getByLabel(/^Password/).fill(process.env.RECOLLECT_OWNER_PASSWORD!);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Your Brains", exact: true })).toBeVisible();
}
const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><defs><linearGradient id="g"><stop stop-color="#28755b"/><stop offset="1" stop-color="#94c5aa"/></linearGradient></defs><rect width="48" height="48" rx="8" fill="url(&quot;#g&quot;)"/><path d="M12 24h24M24 12v24" stroke="white" stroke-width="3"/></svg>`;
const artwork = { name: "mark.svg", mimeType: "image/svg+xml", buffer: Buffer.from(svg) };
const dialog = (page: Page) => page.getByRole("dialog", { name: "Create a Brain", exact: true }).or(page.getByRole("region", { name: "Edit Brain", exact: true }));

test("Brain chooser, safe icon upload, retry without duplicate, replace and remove", async ({ page }) => {
  test.setTimeout(90000);
  await login(page);
  await expect(page.getByTestId("live-pipeline")).toHaveCount(0);
  let attempts = 0;
  await page.route("**/api/brains/*/icon", route => {
    if (route.request().method() === "PUT" && ++attempts === 1) return route.fulfill({ status: 503, json: { message: "Upload retry fixture" } });
    return route.continue();
  });
  const name = `Artwork ${randomUUID().slice(0,8)}`;
  await page.getByRole("button", { name: "Create Brain", exact: true }).click();
  await dialog(page).getByLabel(/^Name/).fill(name);
  await dialog(page).locator('input[type="file"]').setInputFiles(artwork);
  await expect(dialog(page).getByAltText("Brain icon preview")).toBeVisible();
  await dialog(page).getByRole("button", { name: "Create Brain", exact: true }).click();
  await expect(dialog(page).getByText("Upload retry fixture")).toBeVisible();
  await expect(dialog(page).getByLabel(/^Name/)).toBeDisabled();
  let brains = await (await page.request.get("/api/brains")).json();
  expect(brains.filter((b: {name:string}) => b.name === name)).toHaveLength(1);
  await dialog(page).getByRole("button", { name: "Retry icon", exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
  brains = await (await page.request.get("/api/brains")).json();
  const brain = brains.find((b: {name:string}) => b.name === name);
  expect(brain.icon_revision).toBeTruthy();
  expect(brains.filter((b: {name:string}) => b.name === name)).toHaveLength(1);
  const pngResponse = await page.request.get(`/api/brains/${brain.id}/icon`);
  expect(pngResponse.headers()["content-type"]).toBe("image/png");
  expect(pngResponse.headers()["cache-control"]).toBe("no-store");
  const png = await pngResponse.body();
  expect(png.subarray(0,8)).toEqual(Buffer.from([137,80,78,71,13,10,26,10]));
  await page.goto("/");
  const card = page.getByTestId("brain-card").filter({hasText:name});
  await expect(card.locator("img")).toHaveJSProperty("naturalWidth", 256);
  await card.getByRole("link", { name:`Open ${name}`, exact:true }).click();
  await expect(page).toHaveURL(new RegExp(`/brains/${brain.id}/ask`));
  await page.goto(`/brains/${brain.id}/settings`);
  await page.getByRole("button", {name:"Edit Brain",exact:true}).click();
  await dialog(page).getByLabel(/^Name/).fill(`${name} renamed`);
  await dialog(page).getByRole("button", {name:"Save changes",exact:true}).click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(page.getByRole("main").getByText(`${name} renamed`,{exact:true}).last()).toBeVisible();
  await page.getByRole("button", {name:"Edit Brain",exact:true}).click();
  await expect(dialog(page).getByLabel(/^Name/)).toHaveValue(`${name} renamed`);
  // Embedded SVG script/resources are rejected before any upload; the old icon survives.
  await dialog(page).locator('input[type="file"]').setInputFiles({ ...artwork, buffer: Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"><script>window.bad=true</script></svg>') });
  await expect(dialog(page).getByText(/static SVG without scripts/)).toBeVisible();
  await dialog(page).locator('input[type="file"]').setInputFiles({ name:"large.png",mimeType:"image/png",buffer:Buffer.alloc(524289) });
  await expect(dialog(page).getByText(/no larger than 512 KiB/)).toBeVisible();
  await dialog(page).locator('input[type="file"]').setInputFiles({ name:"mark.png",mimeType:"image/png",buffer:png });
  await expect(dialog(page).getByAltText("Brain icon preview")).toBeVisible();
  await dialog(page).getByRole("button", {name:"Save changes",exact:true}).click();
  await expect(dialog(page)).toHaveCount(0);
  expect((await (await page.request.get(`/api/brains/${brain.id}`)).json()).name).toBe(`${name} renamed`);
  // ICO with an embedded PNG is converted through the same image context.
  const ico = Buffer.alloc(22); ico.writeUInt16LE(1,2); ico.writeUInt16LE(1,4); ico.writeUInt16LE(1,10); ico.writeUInt16LE(32,12); ico.writeUInt32LE(png.length,14); ico.writeUInt32LE(22,18);
  await page.getByRole("button", {name:"Edit Brain",exact:true}).click();
  await dialog(page).locator('input[type="file"]').setInputFiles({name:"mark.ico",mimeType:"image/x-icon",buffer:Buffer.concat([ico,png])});
  await expect(dialog(page).getByAltText("Brain icon preview")).toBeVisible();
  await expect(page.getByRole("region", { name: "Edit Brain", exact: true })).toBeVisible();
  await page.screenshot({path:"../.cache/ui/brain-icon-edit.png",animations:"disabled"});
  await dialog(page).getByRole("button", {name:"Save changes",exact:true}).click();
  await expect(dialog(page)).toHaveCount(0);
  await page.getByRole("button", {name:"Edit Brain",exact:true}).click();
  await dialog(page).getByRole("button", {name:"Remove icon",exact:true}).click();
  await dialog(page).getByRole("button", {name:"Save changes",exact:true}).click();
  await expect(dialog(page)).toHaveCount(0);
  expect((await page.request.get(`/api/brains/${brain.id}/icon`)).status()).toBe(404);
});

test("grouped personal Agents omits unused connections but keeps access and pairing reachable", async ({page}) => {
  await login(page);
  const now = new Date().toISOString();
  const shared = { device_id:randomUUID(),name:"My Codex",host_kind:"codex",integration:"plugin",claimed:true,active:true,created_at:now,expires_at:now,last_used_at:now,brains:[{brain_id:randomUUID(),name:"SWEG",last_used_at:now},{brain_id:randomUUID(),name:"Recollect",last_used_at:now}] };
  let failed = false;
  await page.route("**/api/agents", route => failed ? route.fulfill({status:503,json:{message:"Roster unavailable fixture"}}) : route.fulfill({json:{groups:[{user_name:"owner",agents:[shared,{...shared,device_id:randomUUID(),name:"Unused host",brains:[]}]}],hidden_count:3}}));
  await page.goto("/agents");
  await expect(page.getByTestId("agent-brain-group")).toHaveCount(2);
  await expect(page.getByRole("button",{name:"Inspect My Codex",exact:true})).toHaveCount(2);
  await expect(page.getByText("Unused host",{exact:true})).toHaveCount(0);
  await page.getByLabel("Search agents or Brains").fill("SWEG");
  await expect(page.getByTestId("agent-brain-group")).toHaveCount(1);
  await page.getByRole("button",{name:"Inspect My Codex",exact:true}).click();
  await expect(page.getByRole("dialog",{name:"Agent record"})).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByLabel("Search agents or Brains").fill("");
  await page.screenshot({path:"../.cache/ui/agents-grouped-list.png",animations:"disabled"});
  await page.goto("/devices");
  await expect(page).toHaveURL(/\/agents\?access=true/);
  await expect(page.getByRole("dialog",{name:"Access tokens"})).toBeVisible();
  failed=true;
  await page.reload();
  await expect(page.getByRole("dialog",{name:"Access tokens"})).toBeVisible();
  // Pairing and own-account controls do not depend on the roster endpoint.
  const code="0123E567";
  await page.route(`**/api/devices/pairings/${code}`, route => route.fulfill({json:{user_code:code,name:"Own pairing",state:"pending",expires_at:now}}));
  await page.goto(`/devices?code=${code}`);
  await expect(page.getByText(code,{exact:true})).toBeVisible();
  await expect(page.getByRole("button",{name:"Approve device",exact:true})).toBeVisible();
});
