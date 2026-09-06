# Glossary format

Copy the **docs** overview template into the path below. Keep every required field. Then replace the body with this structure. Do not invent fields.

## Paths

Single context. `docs/overview/glossary.md`. Set `id` to `glossary`, `title` to `Glossary`, `domain` to `system`.

Multiple contexts. One file per context at `docs/overview/glossary-<context>.md`. Set `id` to that filename stem, `title` to `{Context} glossary`, `domain` to the context slug.

The context map is `docs/overview/overview-context-map.md`. Set `id` to `overview-context-map`, `kind` to `overview`.

## Body

```md
# Glossary

The language of this context. What it is and why it exists, in one or two sentences.

## Language

**Order**:
A customer's request to buy goods, tracked from placement to completion.
_Avoid_: Purchase, transaction

**Invoice**:
A request for payment sent to a customer after delivery.
_Avoid_: Bill, payment request

**Customer**:
A person or organization that places orders.
_Avoid_: Client, buyer, account
```

## Rules

- **Be opinionated.** When multiple words exist for the same concept, pick the best one and list the others under `_Avoid_`.
- **Keep definitions tight.** One or two sentences max. Define what it is, not what it does.
- **Only include terms specific to this project's context.** General programming concepts do not belong even if the project uses them extensively. Before adding a term, ask whether it is unique to this context. Only then add it.
- **Group terms under subheadings** when natural clusters emerge. If all terms belong to a single cohesive area, a flat list is fine.

## Context map

Use this only when more than one glossary exists.

```md
# Context map

## Contexts

- [[glossary-ordering]] receives and tracks customer orders
- [[glossary-billing]] generates invoices and processes payments
- [[glossary-fulfillment]] manages warehouse picking and shipping

## Relationships

- **Ordering → Fulfillment.** Ordering emits `OrderPlaced` events. Fulfillment consumes them to start picking.
- **Fulfillment → Billing.** Fulfillment emits `ShipmentDispatched` events. Billing consumes them to generate invoices.
- **Ordering ↔ Billing.** Shared types for `CustomerId` and `Money`.
```

Link glossaries with `[[wikilinks]]`. Do not use relative `.md` paths.

A relationship that is hard to reverse, surprising, and a real trade-off is an ADR, not a bullet on this map.
