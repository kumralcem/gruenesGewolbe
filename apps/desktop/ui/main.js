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
  const imported = await invoke("import_paintings", { command: { source_folder: sourceFolder } });
  state.selectedItemId = imported[0]?.id ?? state.selectedItemId;
  await refreshWorkbench();
});

elements.captureForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  await invoke("capture_idea", {
    command: {
      source_link: elements.captureUrl.value.trim(),
      title: elements.captureTitle.value.trim(),
      saving_reason: null,
      copied_text: elements.captureText.value.trim() || null,
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
      home_subvault: state.currentSubvault,
      search_query: state.searchQuery || null,
      selected_item_id: state.selectedItemId,
    },
  });
  renderSnapshot(snapshot);
}

function renderSnapshot(snapshot) {
  state.activeVault = snapshot.active_vault;
  elements.activeVault.textContent = snapshot.active_vault?.root ?? "No vault open";
  elements.currentSubvault.textContent = state.currentSubvault;
  elements.itemCount.textContent = `${snapshot.artwork_items.length} items`;

  renderNav(elements.subvaults, snapshot.subvaults, (subvault) => {
    state.currentSubvault = subvault;
    refreshWorkbench();
  });
  renderNav(
    elements.collections,
    snapshot.collections.map((collection) => collection.name),
    () => {},
  );
  renderArtwork(snapshot.artwork_items);
  renderDenseList(elements.reviewQueue, snapshot.review_queue, "review_status");
  renderDenseList(elements.ideaSources, snapshot.idea_sources, "source_link");
  renderDenseList(elements.searchResults, snapshot.search_results, "home_subvault");
  renderDetails(snapshot.selected_item);
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
    ["Subvault", item.home_subvault],
    ["Review", item.review_status],
    ["Tags", item.tags.join(", ")],
    ["Collections", item.collections.join(", ")],
    ["Source", item.source_link ?? ""],
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
