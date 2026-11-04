oxploy samples what the machine, the apps and oxploy itself take every two seconds, and
keeps the last ten minutes in memory. The charts need nothing installed: oxploy reads
the processes it runs.

## An app

A service's **Monitoring** tab shows the CPU, memory and disk its app takes, with what
the app started counted in. It also shows how often the app exited on its own since its
last deploy or start.

## The server

**Monitoring** in the sidebar, which only the super admin sees, has two tabs:

- **Machine**: the CPU, memory, disk activity, network and disk space of the whole
  server.
- **oxploy**: what the panel takes, with its builds apart, and how much disk its data and
  database use. Show the apps on the same chart to compare them with the panel.

Ten minutes is a window to watch what happens now, such as a deploy or a load test. The
samples start over when oxploy restarts.
