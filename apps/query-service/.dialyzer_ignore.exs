[
  # JOSE library returns {true/false, JWT, JWS}, not {:error, reason}
  # These defensive pattern matches are unreachable per dialyzer
  ~r/user_socket\.ex.*pattern_match/,
  ~r/auth_pipeline\.ex.*pattern_match/,
  ~r/jwt_auth\.ex.*pattern_match/,
  # Pattern match coverage warnings (exhaustive matches flagged by dialyzer)
  ~r/query_controller\.ex.*pattern_match_cov/,
  ~r/tenant_context\.ex.*pattern_match_cov/,
  # WebSocket client no_return (expected — reconnection loops)
  ~r/core_websocket_client\.ex.*no_return/,
  # Mint.WebSocket.new can return {:ok, conn, ws} but Dialyzer infers only error from typespecs
  ~r/core_websocket_worker\.ex.*pattern_match/
]
