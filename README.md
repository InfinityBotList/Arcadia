<h2 align='center'>
  <img src="https://cdn.infinitybots.xyz/images/png/Infinity5.png" height='100px' width='100px' />
  <br> 
  Arcadia
</h2>
<p align="center">
 Arcadia is a monorepo with the following components:
</p>

<hr>

## Project Structure

- ``bot`` => Rewrite of the Discord Bot used for Managing the Infinity Bot List Servers
  - ``leone`` => a library crate containing proc-macros for the bot
- ``api`` => Arcadia Staff API (statistics, maybe a claim/unclaim/approve/deny api for panel if needed etc)
- ``libavacado`` => Common shared logic for ``bot``, ``api`` and any possible addition to Arcadia
- ``_slogjson`` => A vendored fork of the ``slog-json`` crate with some modifications to make it work better with Arcadia

## Database Seeds

You can find a seed of the Infinity Bot List database at https://ptb.botlist.app/dev/seed. This seed is public and available for all contributors

## Contributing

- Always run ``fmt.sh`` before making a Pull Request!
- Always increment version during big changes

## MacOS cross compile

Follow https://stackoverflow.com/questions/40424255/cross-compilation-to-x86-64-unknown-linux-gnu-fails-on-mac-osx

**Use https://github.com/MaterializeInc/homebrew-crosstools for cross compiling as it is newer**

**Path update**

``PATH=/opt/homebrew/Cellar/x86_64-unknown-linux-gnu/0.1.0/bin:$PATH``

**Not always needed, try running ``make cross`` before doing the below**

Symlink ``gcc`` if needed by ring at ``/opt/homebrew/Cellar/x86_64-unknown-linux-gnu/0.1.0/bin`` based on the error you get

Replace 7.2.0 with whatever gcc version you need

``make push`` to push newly built lib. Mofidy according to your ssh ip

**If you face any build issues on macOS, try removing ``/opt/homebrew/bin/x86_64-linux-gnu-gcc`` and then ``ln -sf /opt/homebrew/bin/x86_64-unknown-linux-gnu-cc /opt/homebrew/bin/x86_64-linux-gnu-gcc``

