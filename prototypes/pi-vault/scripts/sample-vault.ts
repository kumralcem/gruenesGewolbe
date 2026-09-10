import { Vault } from "../src/vault.ts";

export async function seedIdeas(vault: Vault) {
  const examples = [
    [
      "Support inbox triage",
      "Classify inbound requests by urgency. Draft responses from the knowledge base. Require a person to review before sending. Escalate refunds and sensitive requests.",
      "A support assistant reads incoming email, labels requests and drafts responses. Human approval prevents accidental commitments.",
      ["support", "inbox", "triage"],
    ],
    [
      "Customer follow-up workflow",
      "After a sales call, extract agreed next steps, prepare an email draft, and add a reminder. Check the recipient and commitments before sending.",
      "Use an agent to manage customer follow-up correspondence and reminders after sales meetings.",
      ["sales", "follow-up"],
    ],
    [
      "Automating a shared mailbox",
      "Connect a read-only mailbox first. Test classification on historical mail. Enable drafting only after inspecting mistakes.",
      "Start customer email automation with read-only access. Do not enable autonomous sending until its behavior is understood.",
      ["mailbox", "automation"],
    ],
    [
      "Painting storage",
      "Store canvases vertically in a dry room. Avoid direct sunlight.",
      "A guide to caring for physical paintings.",
      ["art", "storage"],
    ],
    [
      "Growing basil",
      "Give basil strong light and water when the upper soil is dry.",
      "Household herb cultivation.",
      ["gardening"],
    ],
  ] as const;
  for (const [i, [title, summary, sourceText, tags]] of examples.entries())
    await vault.save({
      kind: "idea",
      title,
      summary,
      sourceText,
      tags: [...tags],
      subvault: "Ideas",
      sourceUrl: `https://fixtures.example/note-${i}`,
      publishedAt: "2026-05-27",
    });
}
