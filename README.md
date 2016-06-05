genv
====

A centralized server for UNIX environment variables.

# Downloading and compiling

You'll need git and rust.

```bash
git clone https://github.com/briansteffens/genv
cd genv
make
sudo make install
```

# Configuring the server

Make a directory for server config:

```bash
sudo mkdir -p /etc/genv
```

It should be readable and writable by the user you're going to run the server
as:

```bash
chmod -hR $USER:$USER /etc/genv
```

Make the config file and set a secret for the server. All clients using this
server will need to know this secret and will use it to authenticate. Use your
preferred text editor to make `/etc/genv/config.json` look like the following:

```json
{
  "secret": "$SOME_SECRET",
}
```

Run the server:

```bash
genv-server
```

By default the server will run as `localhost:3000`. Ideally this should be
behind a reverse proxy like Apache or nginx providing SSL termination.

# Using the client

The client will need to be configured with the server's URL and secret:

```bash
genv config http://localhost:3000/
genv config $SOME_SECRET
```

These values will be stored in `~/.genv.conf`.

Now you can give the server an environment variable:

```bash
genv set HELLO_WORLD "Greetings!"
```

The new value will be stored on the server in the file `/etc/genv/state.json`,
which will look something like this:

```json
{
    "HELLO_WORLD": "Greetings!"
}
```

You can query the server to get the value back:

```bash
genv get HELLO_WORLD
```

Which should return `Greetings!`.

From this or another client, you can pull in all of the server's environment
variables by doing an update:

```bash
genv update
```

This will create a file `~/.genv` with exports for each environment variable,
looking like this:

```bash
export HELLO_WORLD="Greetings!"
```

And it will also include `~/.genv` into `~/.bashrc` by appending the following
to `~/.bashrc`:

```bash
source ~/.genv
```

The new environment variables will not take effect until either a new terminal
is started or `source ~/.genv` is run manually.
