oxploy fetches code through a GitHub App of your own, which it registers for you from the
panel. The app is private to your account or organization, it can only read code, and
you choose which repositories it sees.

## Connect an account

1. Open the panel at the address you will keep using, such as
   [its domain](/docs/installation#give-the-panel-a-domain). GitHub sends the browser back
   to that address after each step.
2. Open **Git** in the sidebar and connect. To register the app on an organization rather
   than your own account, type the organization's name.
3. GitHub asks you to create the app, then to install it. Install it on all your
   repositories or only on some.

Each organization in oxploy connects its own accounts, and its services build from them.

## Choose the repositories

A service picks an account, then one of the repositories the app can read, then a branch.
If a repository is missing from the list, add it to the app's installation on GitHub, in
the installation's settings, and it shows up in the panel.

## Private repositories

Private repositories work like public ones: oxploy fetches them with a short-lived token
of the app's installation, which GitHub gives it for an hour at a time.

Next, [deploy a service](/docs/deploying).
