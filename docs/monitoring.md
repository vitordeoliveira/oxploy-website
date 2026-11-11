oxploy samples what the machine, the apps and oxploy itself take every two seconds, and
keeps the last ten minutes in memory. The charts need nothing installed: oxploy reads
the processes it runs.

## An app

A service's **Monitoring** tab shows the CPU, memory and disk its app takes, with what
the app started counted in, and the latest readings in a table. Above the charts, it says
since when the app runs, its process ID, how many processes it has, how often it exited
on its own since its last deploy or restart, and how much its storage takes.

![A service's Monitoring tab, with its CPU, memory and disk activity over ten minutes](monitoring-app)

Network traffic is not measured per app: the apps share the machine's network.

## The server

**Monitoring** in the sidebar, which only the super admin sees, has two tabs:

- **Machine**: the CPU, memory, disk activity, network and disk space of the whole
  server, with its name, system, processor and how long it has been up.
- **oxploy**: what the panel takes, with its builds apart, and how much disk its data and
  database use. Show the apps on the same chart to compare them with the panel.

![The server's Monitoring page, with the machine's CPU, memory, disk activity and network](monitoring-server)

Ten minutes is a window to watch what happens now, such as a deploy or a load test. The
samples start over when oxploy restarts.
