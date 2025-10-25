(ns allsource.domain.protocols.query-executor
  "Protocol definitions for query execution.
   These are the abstractions that infrastructure will implement.")

(defprotocol QueryExecutor
  "Protocol for executing queries against the event store."

  (execute-query [this query]
    "Execute a query and return results.
     query: Query entity
     Returns: vector of events")

  (execute-query-async [this query callback]
    "Execute a query asynchronously with callback.
     query: Query entity
     callback: function to call with results
     Returns: future or promise")

  (stream-query-results [this query]
    "Execute a query and stream results lazily.
     query: Query entity
     Returns: lazy sequence of events")

  (compile-query [this query]
    "Compile a query to an intermediate representation.
     query: Query entity
     Returns: compiled query (implementation-specific)"))

(defprotocol QueryOptimizer
  "Protocol for query optimization."

  (optimize [this query]
    "Optimize a query before execution.
     query: Query entity
     Returns: optimized Query entity")

  (explain [this query]
    "Explain how a query will be executed.
     query: Query entity
     Returns: execution plan as data"))

(defprotocol QueryValidator
  "Protocol for query validation."

  (validate [this query]
    "Validate a query entity.
     query: Query entity
     Returns: {:valid? boolean :errors vector}")

  (validate-against-schema [this query schema]
    "Validate query fields against event schema.
     query: Query entity
     schema: event schema
     Returns: {:valid? boolean :errors vector}"))
