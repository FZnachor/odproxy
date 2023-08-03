# odproxy

Simple HTTP reverse proxy that dynamically manages backend services based on demand. 

Find yourself ever in a situation where you don't have a lot of resources and you run a lot of different services on your server that nobody uses most of the time? Then odproxy is for you!

## Features

- configurable **listening address** and **port**
- **multiple** configured **hosts** with **multiple hostnames**
- automatic process **spawning** based on demand
- **logging** cases of **unknown hostnames**

### In future releases

- listening on socket
- start and stop command mode