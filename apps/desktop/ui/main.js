const invoke = window.__TAURI__?.core?.invoke ?? fallbackInvoke;

const state = {
  activeVault: null,
  currentSubvault: "Paintings",
  selectedItemId: null,
  searchQuery: "",
};

const elements = {
  activeVault: document.querySelector("#active-vault"),
  vaultForm: document.querySelector("#vault-form"),
  vaultPath: document.querySelector("#vault-path"),
  createVault: document.querySelector("#create-vault"),
  searchForm: document.querySelector("#search-form"),
  searchQuery: document.querySelector("#search-query"),
  importForm: document.querySelector("#import-form"),
  importPath: document.querySelector("#import-path"),
  captureForm: document.querySelector("#capture-form"),
  captureUrl: document.querySelector("#capture-url"),
  captureTitle: document.querySelector("#capture-title"),
  captureText: document.querySelector("#capture-text"),
  subvaults: document.querySelector("#subvaults"),
  collections: document.querySelector("#collections"),
  currentSubvault: document.querySelector("#current-subvault"),
  itemCount: document.querySelector("#item-count"),
  artworkGrid: document.querySelector("#artwork-grid"),
  reviewQueue: document.querySelector("#review-queue"),
  ideaSources: document.querySelector("#idea-sources"),
  detailsList: document.querySelector("#details-list"),
  searchResults: document.querySelector("#search-results"),
};

elements.vaultForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const root = elements.vaultPath.value.trim();
  if (!root) return;
  state.activeVault = await invoke("open_vault", { root });
  await refreshWorkbench();
});

elements.createVault.addEventListener("click", async () => {
  const root = elements.vaultPath.value.trim();
  if (!root) return;
  state.activeVault = await invoke("create_vault", { root });
  await refreshWorkbench();
});

elements.searchForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  state.searchQuery = elements.searchQuery.value.trim();
  await refreshWorkbench();
});

elements.importForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const sourceFolder = elements.importPath.value.trim();
  if (!sourceFolder) return;
  const imported = await invoke("import_paintings", { command: { sourceFolder } });
  state.selectedItemId = imported[0]?.id ?? state.selectedItemId;
  await refreshWorkbench();
});

elements.captureForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await invoke("capture_idea", {
    command: {
      sourceLink: elements.captureUrl.value.trim(),
      title: elements.captureTitle.value.trim(),
      savingReason: null,
      copiedText: elements.captureText.value.trim() || null,
    },
  });
  elements.captureUrl.value = "";
  elements.captureTitle.value = "";
  elements.captureText.value = "";
  await refreshWorkbench();
});

async function refreshWorkbench() {
  const snapshot = await invoke("workbench_snapshot", {
    command: {
      homeSubvault: state.currentSubvault,
      searchQuery: state.searchQuery || null,
      selectedItemId: state.selectedItemId,
    },
  });
  renderSnapshot(snapshot);
}

function renderSnapshot(snapshot) {
  state.activeVault = snapshot.activeVault;
  elements.activeVault.textContent = snapshot.activeVault?.root ?? "No vault open";
  elements.currentSubvault.textContent = state.currentSubvault;
  elements.itemCount.textContent = `${snapshot.artworkItems.length} items`;

  renderNav(elements.subvaults, snapshot.subvaults, (subvault) => {
    state.currentSubvault = subvault;
    refreshWorkbench();
  });
  renderNav(
    elements.collections,
    snapshot.collections.map((collection) => collection.name),
    () => {},
  );
  renderArtwork(snapshot.artworkItems);
  renderDenseList(elements.reviewQueue, snapshot.reviewQueue, "reviewStatus");
  renderDenseList(elements.ideaSources, snapshot.ideaSources, "sourceLink");
  renderDenseList(elements.searchResults, snapshot.searchResults, "homeSubvault");
  renderDetails(snapshot.selectedItem);
}

function renderNav(container, values, onSelect) {
  container.replaceChildren(
    ...values.map((value) => {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = value;
      button.addEventListener("click", () => onSelect(value));
      return button;
    }),
  );
}

function renderArtwork(items) {
  elements.artworkGrid.replaceChildren(
    ...items.map((item) => {
      const tile = document.createElement("button");
      tile.type = "button";
      tile.className = "artwork-tile secondary";
      tile.addEventListener("click", async () => {
        state.selectedItemId = item.id;
        await refreshWorkbench();
      });
      tile.innerHTML = `
        <div class="thumb">${escapeHtml(item.title.slice(0, 1) || "?")}</div>
        <div class="tile-copy">
          <strong>${escapeHtml(item.title)}</strong>
          <span>${escapeHtml([item.creator, item.year].filter(Boolean).join(" - "))}</span>
        </div>
      `;
      return tile;
    }),
  );
}

function renderDenseList(container, items, secondaryField) {
  container.replaceChildren(
    ...items.map((item) => {
      const row = document.createElement("li");
      row.innerHTML = `
        <strong>${escapeHtml(item.title ?? item.id)}</strong>
        <span>${escapeHtml(item[secondaryField] ?? "")}</span>
      `;
      return row;
    }),
  );
}

function renderDetails(item) {
  if (!item) {
    elements.detailsList.replaceChildren();
    return;
  }

  const rows = [
    ["Title", item.title],
    ["Creator", item.creator],
    ["Year", item.year],
    ["Subvault", item.homeSubvault],
    ["Review", item.reviewStatus],
    ["Tags", item.tags.join(", ")],
    ["Collections", item.collections.join(", ")],
    ["Source", item.sourceLink ?? ""],
  ];

  elements.detailsList.replaceChildren(
    ...rows.flatMap(([label, value]) => {
      const term = document.createElement("dt");
      const detail = document.createElement("dd");
      term.textContent = label;
      detail.textContent = value;
      return [term, detail];
    }),
  );
}

function fallbackInvoke(command) {
  throw new Error(`Tauri command unavailable: ${command}`);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
