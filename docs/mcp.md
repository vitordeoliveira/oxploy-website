The panel answers the [Model Context Protocol](https://modelcontextprotocol.io) at `/mcp`,
such as `https://panel.example.com/mcp`, so AI agents such as Claude Code or a bot can read
the server and call the tools of your apps. Each client gets a token of its own on the
**MCP** page, in the sidebar, and sends it as `Authorization: Bearer <token>`. What it
reaches depends on the token: the server, or one organization's apps.

![The MCP page, with the endpoint, a server token, and a token of the Acme organization](mcp-page)

oxploy shows a token once, and keeps only its hash. A missing or wrong token gets a 401
after a second, and **Revoke** turns its client away. Make one token per client, so you
can revoke one without the others. Two tokens of the server, or of one organization,
cannot have names that read the same, such as `Claude Local` and `claude-local`.

## Server tokens

A server token, which only the super admin makes, reads the machine, oxploy itself and
every app it runs, across organizations:

| Tool | What it returns |
| --- | --- |
| `server_status` | The machine's CPU, memory, disk and network; oxploy's version, updates and the one being installed, and Caddy; how many apps stand where |
| `list_services` | Every app, with its status, port, domains, release, usage now, crashes and latest deployment |
| `service_usage` | One app's process, its CPU and memory now and over the last 10 minutes, its crashes and its storage |
| `app_logs` | The end of an app's log, or only its lines with a text |
| `list_deployments` | An app's latest deployments |
| `deployment_log` | The end of a deployment's log |
| `list_mcp_tokens` | The server's tokens and each organization's, with the tools turned on in its apps and the ones each token reaches, never their secrets |
| `app_mcp_tools` | The tools an app's MCP server offers, what each declares it does, and which are turned on |

![The New server token dialog, with Can manage off](mcp-server-token)

A server token made with **Can manage** on also gets the tools that change something, and
its row says so:

| Tool | What it does |
| --- | --- |
| `update_oxploy` | Installs oxploy's newest release, as the Update button does, and answers at once |
| `set_app_tools` | Turns tools of an app's MCP server on or off, as its MCP tab does |
| `set_token_tools` | Has an organization's token reach every tool turned on, or only the ones given |
| `create_mcp_token` | Makes an organization's token with the tools picked, and returns its secret and the commands that connect Claude Code once |
| `revoke_mcp_token` | Revokes an organization's token |

Server tokens are only made and revoked on the page, so an agent cannot make itself a
token that manages, and the tools that manage reach only the organizations' tokens. A
token that only reads gets the same error for them as for a tool that does not exist.
Clients such as Claude Code ask you before each call unless told not to, and oxploy logs
each call to a tool that manages with the token, the tool, how it went and how long it
took.

## Organization tokens

An organization's token reaches the tools of its apps that serve MCP, the ones turned on
in each app's MCP tab: see [Apps' MCP servers](/docs/app-mcp). Each tool is named after
its app, such as `shop-docs__search_docs`, and a call goes to the app's own server. An app
that is not running lists none, and other organizations' tokens see none of them.

![The New token dialog of an organization, with two of its tools picked and Available to this organization's apps on](mcp-org-token)

A token reaches every tool turned on, including the ones turned on later, or only the
ones picked for it. A new token starts with the ones picked in the dialog that makes it,
none unless some are; its **Tools** button changes them later, and its row warns when it
reaches no tool. Turning a tool off in the app's tab takes it from every token, and
turning it back on gives it back to the tokens that had it picked. A call to a tool the
token does not reach gets the same error as a tool that does not exist.

oxploy logs each call an organization's token makes, with the token, the tool, how it
went and how long it took, but not its arguments, which may hold people's data.

## Connect Claude Code

With a new token, the page gives the commands that connect Claude Code to it, under a
name made from the token's: `oxploy-server-<token>` for a server token, or
`oxploy-<organization>-<token>` for an organization's, so a client that holds several
tells them apart. For a server token named Claude Local:

```sh
claude mcp add --transport http --scope local oxploy-server-claude-local https://panel.example.com/mcp \
  --header "Authorization: Bearer <token>"
```

`--scope local` adds it to the project Claude Code runs in, and `--scope user` to every
project of its user; both keep it in `~/.claude.json`. Neither is `--scope project`,
which would write the token into the repository's `.mcp.json`. Run them in a terminal of
your own: in a Claude conversation, the token would stay in its history.

## Tokens for apps

A client that runs on the server as one of the organization's apps, such as a bot, takes
its token as a variable. Turn **Available to this organization's apps** on when you make
the token: oxploy then keeps the token itself, not only its hash, and the app takes it as
a variable named after it, such as `${OXPLOY_MCP_TELEGRAM_BOT}`, which the token's row
shows. In the app's [Environment tab](/docs/deploying#environment-variables):

```sh
MCP_TOKEN=${OXPLOY_MCP_TELEGRAM_BOT}
MCP_URL=${OXPLOY_MCP_URL}
```

Every app gets `OXPLOY_MCP_URL`, where this server's MCP answers on `127.0.0.1`. Only an
app whose variables use a token gets it; saving the variables checks that the token
exists, and one revoked since is empty, without the app losing its other variables. A
token made before oxploy kept its secret is made available with **For apps**, which gives
it a new one. Server tokens are never available to apps.

## These docs over MCP

This documentation has an MCP server of its own, which anyone's agent can search and
read:

```sh
claude mcp add --transport http oxploy-docs https://oxploy.com/mcp
```
