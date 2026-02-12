what we shipped this week on allsource

had a weird problem. our stack is go + rust + elixir. but our mcp server? typescript. it stuck out like a sore thumb

so we rewrote it in elixir. took a day. feels way better now

but here's where it gets interesting. we also discovered TOON format

TOON is built for LLMs. same data as JSON but 50% fewer tokens. when you're paying per token to claude/gpt, that matters

look at this difference:

JSON:
{"events": [{"id": "evt-1", "type": "created"}, {"id": "evt-2", "type": "updated"}]}

TOON:
events[2]{id,type}:
  evt-1,created
  evt-2,updated

half the tokens. LLMs understand it perfectly

we made it smart. tabular data? automatically uses TOON. complex nested stuff? falls back to JSON. you don't even think about it

implementation was straightforward. added toon_ex lib, wrote a detector, updated handlers. maybe 4 hours total

but here's the thing: elixir version is actually superior

OTP supervision = won't randomly die
pattern matching = clean code
matches our query service patterns
TOON baked in from day one

we nuked the old typescript code. didn't need it. cleaner is better

the mcp server connects claude desktop to our event store. so when you ask "what changed for user-123 yesterday?" it queries our temporal data

now those responses are 50% smaller. less API calls. lower costs. faster processing

updated all docs too. setup guides point to elixir now. README cleaned up. old references gone

total migration: 2 days. sets us up for massive token savings going forward

thinking about adding TOON to rust core API next week. optional format param. could amplify the savings

mcp server is the bottleneck though. every response hits it. so optimizing there = biggest win

if you're building LLM tools, check out TOON. it's literally designed for this. github has the spec

allsource is open source. temporal event store built for AI apps. if you're doing events + LLMs, might be interesting

/end
