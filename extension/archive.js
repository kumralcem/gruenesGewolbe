const $ = (id) => document.getElementById(id);
export function setupArchive(request, connection) {
  let nextOffset = null;
  async function search(append = false) {
    try {
      const page = await request(
        `/archive?q=${encodeURIComponent($("archive-query").value)}&offset=${append ? nextOffset : 0}`,
      );
      if (!append) $("archive-records").replaceChildren();
      for (const record of page.records) {
        const li = document.createElement("li");
        li.textContent = `${record.title} — ${record.subvault}`;
        for (const originals of [false, true]) {
          const button = document.createElement("button");
          button.className = "secondary";
          button.textContent = originals ? "Originals only" : "Download bundle";
          button.onclick = async () => {
            button.disabled = true;
            try {
              const { url, token } = connection();
              const response = await fetch(
                `${url}/archive/${record.id}/download?originals=${originals ? 1 : 0}`,
                {
                  headers: { authorization: `Bearer ${token}` },
                  redirect: "error",
                  signal: AbortSignal.timeout(300000),
                },
              );
              if (!response.ok)
                throw Error((await response.json()).error ?? "Download failed");
              const blob = await response.blob();
              const link = document.createElement("a");
              link.href = URL.createObjectURL(blob);
              link.download = `${record.id}${originals ? "-originals" : ""}.tar.gz`;
              link.click();
              setTimeout(() => URL.revokeObjectURL(link.href), 60000);
              $("archive-status").textContent =
                "Downloaded. Extract the bundle to open its files.";
            } catch (error) {
              $("archive-status").textContent = error.message;
            } finally {
              button.disabled = false;
            }
          };
          li.append(button);
        }
        $("archive-records").append(li);
      }
      nextOffset = page.nextOffset;
      $("archive-more").hidden = nextOffset === null;
      $("archive-status").textContent = page.records.length
        ? "Bundles include Markdown, preserved source and original files."
        : "No matching records.";
    } catch (error) {
      $("archive-status").textContent = error.message;
    }
  }
  $("archive-search").onsubmit = (event) => {
    event.preventDefault();
    void search();
  };
  $("archive-more").onclick = () => search(true);
  let files = [],
    stopping = false;
  $("import-files").onchange = () => {
    files = [...$("import-files").files];
  };
  $("import-drop").ondragover = (event) => event.preventDefault();
  $("import-drop").ondrop = (event) => {
    event.preventDefault();
    if ($("import-start").disabled) return;
    files = [...event.dataTransfer.files];
    $("import-status").textContent = `${files.length} file(s) selected.`;
  };
  $("import-stop").onclick = () => {
    stopping = true;
    $("import-stop").disabled = true;
  };
  $("import-start").onclick = async () => {
    if (!files.length) {
      $("import-status").textContent = "Choose images first.";
      return;
    }
    stopping = false;
    $("import-start").disabled = true;
    $("import-files").disabled = true;
    $("import-stop").disabled = false;
    const instructions = $("import-instructions").value.trim() || undefined;
    try {
      for (const [index, file] of files.entries()) {
        if (stopping) break;
        const report = (text) => {
          $("import-status").textContent =
            `${index + 1}/${files.length}: ${file.name} — ${text}`;
        };
        report("preparing");
        if (
          !file.size ||
          file.size > 64000000 ||
          !/\.(jpe?g|png|webp)$/i.test(file.name)
        )
          throw Error(
            `${file.name}: expected JPEG, PNG or WebP, at most 64 MB`,
          );
        const bytes = new Uint8Array(await file.arrayBuffer());
        const hash = Array.from(
          new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
          (b) => b.toString(16).padStart(2, "0"),
        ).join("");
        let binary = "";
        for (let start = 0; start < bytes.length; start += 32768)
          binary += String.fromCharCode(
            ...bytes.subarray(start, start + 32768),
          );
        const mimeType = file.name.toLowerCase().endsWith("png")
          ? "image/png"
          : file.name.toLowerCase().endsWith("webp")
            ? "image/webp"
            : "image/jpeg";
        const url = `gg-local:sha256:${hash}`;
        const capture = {
          version: 2,
          intent: "capture",
          url,
          title: file.name,
          capturedAt: new Date().toISOString(),
          text: `Local image: ${file.name}. Preserve the supplied original image. The filename is context, not verified attribution.`,
          instructions,
          images: [{ url, mimeType, bytes: btoa(binary), alt: file.name }],
        };
        const digest = Array.from(
          new Uint8Array(
            await crypto.subtle.digest(
              "SHA-256",
              new TextEncoder().encode(
                JSON.stringify({ ...capture, capturedAt: undefined }),
              ),
            ),
          ),
          (b) => b.toString(16).padStart(2, "0"),
        ).join("");
        const id = `${digest.slice(0, 8)}-${digest.slice(8, 12)}-4${digest.slice(13, 16)}-a${digest.slice(17, 20)}-${digest.slice(20, 32)}`;
        let job = await request("/captures", {
          method: "POST",
          headers: {
            "content-type": "application/json",
            "x-gg-capture-id": id,
          },
          body: JSON.stringify(capture),
        });
        if (
          ["failed", "partial", "interrupted", "cancelled"].includes(job.status)
        )
          job = await request(`/captures/${job.id}/retry`, { method: "POST" });
        while (["pending", "running"].includes(job.status)) {
          report(job.status + "; keep this page open to send remaining images");
          await new Promise((resolve) => setTimeout(resolve, 2000));
          job = await request(`/captures/${job.id}`);
        }
        if (job.status !== "completed")
          throw Error(
            `${file.name}: ${job.status}. ${job.error ?? job.result?.outcome?.reason ?? "Check recent captures."} Select the same files again to resume.`,
          );
        report("saved");
      }
      $("import-status").textContent = stopping
        ? "Stopped. Select the same files again to resume; saved originals are skipped."
        : "Import finished.";
    } catch (error) {
      $("import-status").textContent = error.message;
    } finally {
      $("import-start").disabled = false;
      $("import-files").disabled = false;
      $("import-stop").disabled = true;
    }
  };
}
