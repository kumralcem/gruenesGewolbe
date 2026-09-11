---
status: accepted
---

# One Selected Image per Visual Capture

A Visual Capture preserves one Selected Image and researches better copies of that same image, including for photography and sculpture. It does not substitute another photograph, gather alternative viewpoints, or automatically group several depictions of an artwork, because the user's chosen image is the intended material and a model for multiple depictions would add unwanted complexity to the first version.

For X, capture targets the image the user supplies from the main post; quoted posts and replies require their own explicit captures. A source with no unambiguous single image enters the Capture Queue for the user to select the intended image later. It does not implicitly expand into several saved items or let the agent guess the selection. Research may use other images as evidence without adding them to the saved item.

Retain both the initially obtained file and the chosen better copy of the same Selected Image in one saved item. The user considers the additional storage acceptable at the expected archive size. Selecting a higher-quality Primary File does not delete or overwrite previously preserved versions.

When a repeated capture is confidently matched to an existing Selected Image, use a demonstrably higher-quality copy as that item's Primary File; otherwise leave the item unchanged and return its location. A confirmed match should not create another saved item. This permits automatic primary-file upgrades for better copies of the same image; uncertain matches and unrelated metadata changes are not covered by this permission. The implementation conservatively compares normalized pixel hashes and increasing dimensions, retaining earlier files. Different encodings can remain duplicates when identity cannot be confirmed.
