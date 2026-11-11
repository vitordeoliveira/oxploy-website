An app can serve an MCP server of its own, whose tools then reach its organization's
[MCP](/docs/mcp), so one token gives a client the tools of all your apps. oxploy calls each
app's server itself, on the machine, and you choose which of its tools pass on. An MCP
server that runs elsewhere can join the same way.

## Serve MCP from an app

The app serves MCP over Streamable HTTP on its own port, at a path such as `/mcp`. oxploy
takes answers in plain JSON or as an event stream, with or without a session, so a server
that answers each POST with one JSON-RPC message in plain JSON is enough. In the service's
**MCP** tab:

1. Turn on **This app serves MCP**, and give the path, `/mcp` unless the app answers
   elsewhere, such as `/api/mcp`.
2. Restart the app. oxploy gives it a token in `OXPLOY_MCP_TOKEN` when it starts, and
   calls its server at `127.0.0.1` with it, as `Authorization: Bearer <token>`.
3. The tab says whether it reaches the server, and lists its tools.

![The MCP tab of an app connected to its MCP server, at the path /admin/mcp](app-mcp)

The app should answer only the requests that send the token. One that reads another
variable takes it with a line such as `MCP_TOKEN=${OXPLOY_MCP_TOKEN}` in its
[Environment tab](/docs/deploying#environment-variables). Caddy answers 404 at the path,
and below it, on the app's domains, so only oxploy reaches the server.

```rust
fn is_authorized(authorization: Option<&str>) -> bool {
    let Ok(token) = std::env::var("OXPLOY_MCP_TOKEN") else {
        return false;
    };
    // Compare in constant time in a real app.
    !token.is_empty() && authorization == Some(format!("Bearer {token}").as_str())
}
```

## Turn tools on

The **Tools** card lists the tools the app's server offers now. Each starts off: the ones
you turn on reach the organization's MCP, named after the app, such as
`shop-docs__search_docs`, through the tokens that reach every tool turned on or that have
it picked. Turning one off takes it from every token.

![The Tools card, with three tools turned on](app-mcp-tools)

The card groups the tools by what the app declares they do to its data, in the
protocol's annotations: **Read-only**, **Writes**, or **Destructive**, where a tool that
declares nothing counts as destructive, as the protocol says. While the app is not
running, oxploy cannot ask, and lists none of its tools to the organization.

Clients list the tools when they connect, so the ones already connected see a change once
they reconnect.

## External MCP servers

An MCP server that runs elsewhere, such as one of an app hosted on another platform,
joins an organization as a service of its own kind: **New service**, then **External MCP
server**, with the URL it answers at, such as `https://billing.example.com/mcp`.

![The New service dialog, with External MCP server chosen](external-mcp)

Give the token it takes as `Authorization: Bearer <token>`, or leave it empty and oxploy
makes one, to put in the server's configuration. The service's page is its MCP tab alone,
with the URL and the token, which can be shown, changed or made anew, and its tools, which
are turned on and picked for tokens as an app's are.

oxploy calls it over HTTPS only, follows no redirect, which would carry the token
elsewhere, and never calls an address of its own machine or of a private network,
whatever the URL's host resolves to. A server that cannot be reached lists no tools until
it can. Servers that need a sign-in, such as OAuth, are not supported.
