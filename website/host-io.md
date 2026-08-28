---
title: host I/O
section: learn
status: not-yet
---

# host I/O

Host I/O is how a Program talks to the machine: process, stdio, filesystem, sockets, then thin HTTP.

Networking is sockets-first. TCP listen, accept, connect, read, and write land on native. HTTP/1.1 helpers sit on those sockets. The designed surface is not a Node-shaped `http` module as the only entry.

Listen and server paths are native first. The JS backend hard-errors unsupported host APIs until an explicit bridge exists.

Filesystem, env, and process APIs are the other host class. The public site generator itself is a native Program that reads and writes files.

Continue to [packages](packages.html).
