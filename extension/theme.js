const preference = document.getElementById("theme");
const system = matchMedia("(prefers-color-scheme: dark)");
let choice = "dark";
function apply(value) {
  choice = ["dark", "light", "system"].includes(value) ? value : "dark";
  document.documentElement.dataset.theme =
    choice === "system" ? (system.matches ? "dark" : "light") : choice;
  if (preference) preference.value = choice;
}
apply((await chrome.storage.local.get("theme")).theme);
preference?.addEventListener("change", async () => {
  apply(preference.value);
  await chrome.storage.local.set({ theme: choice });
});
system.addEventListener("change", () => apply(choice));
chrome.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes.theme) apply(changes.theme.newValue);
});
