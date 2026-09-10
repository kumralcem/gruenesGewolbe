# Representative Visual Capture Sources

Supplied by the user and inspected on 2026-09-10. These are candidate evaluation cases, not completed captures. No source media was saved into the live vault, and no Pi integration has been tested.

## Commons painting

[Le Chevalier aux Fleurs file page](https://commons.wikimedia.org/wiki/File:Le_Chevalier_aux_Fleurs_1894_Georges_Rochegrosse_1859_1938.jpg)

The Commons page identifies Georges Rochegrosse's painting and exposes an original JPEG at 2,647 × 1,686 pixels, plus museum references. Candidate checks: obtain the actual image rather than a page screenshot; distinguish the painting's creator and date from the reproduction photographer and upload date; preserve provenance; choose Paintings.

## Wikipedia media selection

[Tancrède Bastet article with selected media](https://fr.wikipedia.org/wiki/Tancr%C3%A8de_Bastet#/media/Fichier:L_atelier_de_Cabanel_a_l_ecole_des_Beaux_Arts.jpg)

The supplied fragment selects a particular artwork within a biography. Its [Commons file page](https://commons.wikimedia.org/wiki/File:L_atelier_de_Cabanel_a_l_ecole_des_Beaux_Arts.jpg) identifies Tancrède Bastet's 1883 painting and exposes a 2,820 × 2,241 JPEG. Candidate checks: honor the selected image rather than choosing the biography's lead portrait; follow through to the file; keep the original source link including its selection information; choose Paintings.

## X painting repost

[soli post](https://x.com/solisolsoli/status/2092378489093595354)

Browser inspection showed a painting image and a caption naming Tigers in the Night and Erik Olson. Those names are source claims, not independently verified attribution. Candidate checks: use the main post's media, use its caption as an identification lead, research a better copy and corroborating attribution, and exclude reply images and discussion.

## X photography repost

[Evolve post](https://x.com/EvolveWildlife/status/2097333486659260893)

The user identifies this example as photography. Browser inspection showed an underwater manatee photograph; the post credits Sylvie Ayer and describes Florida's Homosassa River. These details have not been independently verified. Candidate checks: choose an existing Photography subvault, distinguish photographer from reposting account, and seek a better copy of this exact photograph rather than substitute another image of a manatee.

## X sculpture with a quoted post

[Fëanor post](https://x.com/Noldorcitizen/status/2097248441860497708)

The user identifies this example as sculpture. Browser inspection showed the main post's sculpture image, a quoted post with a separate media link, and replies containing further images. The main caption does not identify the sculpture. Candidate checks: research from the actual intended image, choose an existing Sculptures subvault rather than Photography merely because the representation is a photograph, keep uncertain identity unresolved, and save only the Selected Image from the main post. Exclude quoted and reply media; any improved file must represent the same photograph and viewpoint, without adding other views.

## Access observation

The web reader opened the Wikimedia and Wikipedia pages but returned errors for all three X URLs. All three X posts and their main images were readable in the available in-app browser without signing in during this inspection. This is evidence that a browser fallback is worth testing, not a guarantee of unauthenticated X access in a standalone Pi CLI. Download fidelity, authentication fallback, and research limits still require a working prototype.

## Existing-archive acceptance fixtures

The user recalled format/size problems in the previous workflow. A read-only census of `/home/cem/Gewolbe/subvaults/Paintings/items/*/files/*` found 132 preserved images: 14 WebP, 109 JPEG, and 9 PNG. Pillow successfully read all image headers; this does not prove that full decoding, conversion, or model inspection succeeds.

The vault's `activity-log.tsv` records preview failures for these files because their dimensions exceed the old 24,000,000-pixel limit:

- `Cavalcade-David_Rudnick.webp`: 8,000 × 8,000 pixels, 7,153,234 bytes.
- `Jean-Léon_Gérôme_015_Carpets.jpg`: 4,946 × 6,326 pixels, 6,255,293 bytes.

Their item entries in the latest enrichment checkpoint both say that no real Thumbnail Preview was available and no image was sent to the provider. The checkpoint has 106 enriched items and 26 failed items overall; the other 24 failures report an incomplete OpenAI artwork response. Those provider failures have not been reproduced or diagnosed further, and should not be attributed to WebP.

`Caspar_David_Friedrich_-_Wanderer_above_the_Sea_of_Fog(1818).jpeg` is another large fixture: 5,256 × 6,742 pixels and 31,235,405 bytes. It also exceeds the current pixel threshold but is not one of the two missing-preview entries; current header size alone does not establish a historical failure for that file.

The old pipeline in `crates/archive-core/src/lib.rs` rejects dimensions above the threshold before full decoding, then creates at most a 480 × 480 PNG for accepted images. `apps/desktop/src/artwork_enrichment.rs` sends that PNG to the model and separately limits preview bytes. WebP is an enabled decoder format in `crates/archive-core/Cargo.toml`. These are source and persisted-log observations, not a completed reproduction of the user's earlier experience.

Required prototype checks: preserve original bytes; prepare a usable analysis image from real WebP and large JPEG sources with bounded resource use; verify full image preparation rather than just copying or reading headers; keep failures local to the affected capture and visible for follow-up; distinguish retrieval, decoding, model-request, and response-completion failures; permit retry without making duplicate saved items. The exact image-preparation implementation and model settings remain undecided.
