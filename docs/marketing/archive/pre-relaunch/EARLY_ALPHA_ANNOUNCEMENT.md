# Early Alpha Announcement — LinkedIn + X.com

## LinkedIn Post

---

**Looking for early alpha users for AllSource — an event store where AI agents pay for their own data access.**

We've been building AllSource for the past year: a purpose-built event store in Rust that stores every state change as an immutable event and lets you query any point in history.

The numbers: 469K events/sec ingestion. 11.9us p99 query latency. WAL + Parquet durability — nothing is ever lost.

But what makes it different from EventStoreDB or Kafka isn't speed. It's this:

**Built-in x402 micropayments.** AI agents can consume your API and pay per call automatically via Coinbase's x402 protocol on Base. No wallet SDK. No crypto knowledge on the agent side. Your event store pays for itself.

**43 MCP tools.** Connect Claude Desktop and your agent can query events, build projections, manage schemas, and monitor health — all through natural language.

**We're looking for 10-15 alpha users who:**
- Build with event sourcing (or want to start)
- Run AI agents that need durable memory
- Want to give feedback that shapes the product

**What you get:**
- Free Pro tier for the alpha period (1M events/month, normally $29/mo)
- Direct Slack/Discord access to the team
- Your use case influences the roadmap

The stack is live: https://all-source.xyz
API: https://api.all-source.xyz
Status: https://www.all-source.xyz/status
GitHub: https://github.com/all-source-os/all-source

**One curl to get started — no signup form:**

```
curl -X POST https://api.all-source.xyz/api/v1/onboard/start \
  -H "Content-Type: application/json" \
  -d '{"email":"you@company.com","name":"Your Name"}'
```

DM me or comment below if you're interested. Happy to jump on a call to understand your use case.

#EventSourcing #AI #Rust #OpenSource #DevTools #x402

---

## X.com Post (single post, not thread)

---

Looking for 10-15 early alpha users for AllSource.

Event store in Rust. 469K events/sec. 11.9us queries. 43 MCP tools for Claude/GPT.

The twist: built-in x402 micropayments. AI agents pay per API call automatically. Your event store pays for itself.

One curl to start:
curl -X POST https://api.all-source.xyz/api/v1/onboard/start -d '{"email":"you@co.com"}'

Looking for people who:
→ build with event sourcing
→ run AI agents that need memory
→ want to shape the product

Free Pro tier ($29/mo value) for alpha users.

DM me or reply.

https://all-source.xyz

---

## X.com Alternative (shorter, punchier)

---

We built an event store where AI agents pay for their own data access.

469K events/sec. 11.9us queries. x402 micropayments on Base.

Looking for 10 alpha users. Free Pro tier. DM me.

https://all-source.xyz

---

## Notes for posting

- **LinkedIn**: post from Decebal's personal profile, not a company page. Personal posts get 5-10x more reach.
- **X.com**: use the shorter version if you want engagement. Use the longer one if you want qualified leads.
- **Timing**: Tuesday-Thursday, 8-10am ET for LinkedIn. 11am-1pm ET for X.
- **Follow-up**: reply to every comment within 2 hours. Each reply boosts the post in the algorithm.
- **CTA**: "DM me" converts better than "sign up at link" for alpha — it's lower friction and lets you qualify users.
- **Hashtags**: LinkedIn only. Never on X.
