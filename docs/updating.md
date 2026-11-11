oxploy looks for a new release on GitHub every hour. Once one is out, the super admin sees
**Update available** at the foot of the sidebar.

## Update from the panel

Open **Update available** to see the new version and a link to its release notes, then
press **Update now**. oxploy:

1. Downloads the release's build and checks it against the release's checksum.
2. Runs the new binary once, to check that it works on this machine.
3. Waits for the deployments in progress to finish.
4. Puts the new binary and assets in place of the old ones, and keeps those in
   `previous`.
5. Runs the new version in its own place, with the same process ID.

The apps keep running, and the pages you have open reload on the new version. If a step
fails, the panel says why, keeps running the version it had, and offers to try again.

**Settings** shows the version the panel runs and the latest release, and **Check for
updates** asks GitHub for a new one on demand, without reloading the page.

![The Version card in Settings, up to date](settings-version)

## The folder must be oxploy's

The panel replaces its own files, so its user must own the folder it is installed in, as
the [installation](/docs/installation#install-oxploy) sets it up:

```sh
sudo chown -R oxploy: /opt/oxploy
```

Until then, the panel shows this command instead of updating.

## Go back to the previous version

oxploy keeps the version an update replaced in `previous`, beside the binary. To go back:

```sh
sudo systemctl stop oxploy
cd /opt/oxploy
sudo -u oxploy sh -c 'rm -rf assets oxploy && mv previous/assets previous/oxploy .'
sudo systemctl start oxploy
```

A new version may have changed oxploy's database, which the older one may not read. Back
up `/var/lib/oxploy/oxploy.db` before you go back.

## Update by hand

Stop the service, download the release over the installed one, as in the
[installation](/docs/installation#install-oxploy), and start it again. The apps keep
running meanwhile.

```sh
sudo systemctl stop oxploy
curl -L https://github.com/vitordeoliveira/oxploy/releases/latest/download/oxploy-x86_64-linux.tar.gz | sudo -u oxploy tar xz -C /opt/oxploy
sudo systemctl start oxploy
```
