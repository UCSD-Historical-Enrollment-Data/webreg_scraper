# Docker
This directory contains Dockerfiles that can be used to run the scraper and any 
other accompanying services as well as a basic Docker Compose.

The specifics of these services can be found in their respective README files:
| Service Name | Information |
| ------------ |-------------|
| [`webregautoin`](../scripts/webregautoin/README.md) | A basic web server designed to automatically log the scraper into WebReg. **This is required.** |
| [`webreg`](../crates/webreg/README.md) | This is the actual scraper _and_ API application. |

## Why Docker?
This is the recommended way to run the scraper, as it abstracts away all the
dependency management and setup that would otherwise be required to run the scraper.

## Requirements
In order to ensure that you _can_ use this Docker setup, ensure that the following 
technical and non-technical requirements are satisfied.

### Non-Technical
- You must have a UCSD account that gives you access to WebReg.
- Your UCSD account must be configured so that a Duo push is automatically sent
    when needed (i.e., set Duo Push as the **default authentication method**). See [this UCSD ITS article](https://support.ucsd.edu/its?id=kb_article_view&sys_kb_id=f91e1f55873259d8947a0fa8cebb352e&sysparm_article=KB0030238) for more information.

### Technical
- You must have [Docker](https://www.docker.com/) installed and running on your machine. 
Docker Compose is also required, but it should come bundled with Docker. 
- I strongly recommend having `tmux` or some other terminal multiplexer installed
to make it easier to manage steps 3 and 4 in the setup process below, but it is
not strictly required. You can also just open up multiple terminal windows.
- Some basic familiarity with Docker and the command line will be helpful. Refer
to the [troubleshooting](#troubleshooting) section below.


## Setup 
To run the scraper using Docker, follow the instructions below.
1. Clone this repository and navigate to the `docker` directory.
2. You'll need to create a `config.json` file in the `data` directory. An example
    `config.example.json` file is provided for reference. More details can be found here: [webreg scraper README](../crates/webreg/README.md#configuration-file).
2. Run the command
    ```bash
    docker compose up -d
    ```
    *This is also the command you will use to start the scraper in the future, so remember it.*
> [!NOTE]
> The first time you run this command, it will take a while to build the Docker
> images for the scraper and the webregautoin service, so be patient. Subsequent runs will be much faster.
3. In a separate terminal, run the command
    ```bash
    docker attach login
    ```
    This will attach your terminal to the `webregautoin` container, allowing you to
    input your login credentials. Follow the instructions in the terminal to log in. 
    Once you are done with the login process, you can detach from the container by pressing `Ctrl + P + Q`.
4. The scraper should now be running and automatically logging in when needed. 

## Implementation Details
Two containers are used in this setup, `login` and `scraper`.

The `login` container runs the `webregautoin` script. This script logs you into 
WebReg and serves cookies to the `scraper` container whenever they are needed. 
This is currently being built from the `node:lts-slim` image. 

The `scraper` container runs the actual scraper. It is built from the `rust:slim-trixie`
image. 

### Compose
The compose file is pretty straightforward. It just defines the two services and
sets up a shared network and a volume to store scraped data into. You can modify
it as you see fit, but the current setup should get you up and running. 

I highly recommend keeping the network configuration in [bridge mode](https://docs.docker.com/engine/network/drivers/bridge/) in order to 
ensure that you aren't exposing your cookies outside of where strictly necessary.
You can read more about Docker networking [here](https://docs.docker.com/network/).
Admittedly this is a pretty basic setup, but it does the job. 

Currently, the `scraper` container is configured to mount the `data` directory 
as a volume so that scraped data is stored on your local machine. This is also 
where the `config.json` file is stored, which is required for it to run. 

## Troubleshooting
If you run into any Docker issues these commands might come in handy:
- `docker compose up --force-recreate` - Recreate the containers without rebuilding
the images. This is useful if you want to reset the *state* of the containers. You 
might want to do this if you see a message that says that a container is unhealthy 
or if you see a message that says "Container failed to start" but you aren't sure why. 
- `docker compose logs` - View the logs for all containers. You can also specify
a specific container, e.g., `docker compose logs scraper`.
- `docker compose down` - Stop and remove all containers. You can also specify
a specific container, e.g., `docker compose down scraper`.
- `docker compose up -d --build` - Rebuild the images and restart the containers. 
You might want to do this after a new release or if you make any changes to the 
Dockerfiles yourself.
might want to run this if you see a message that says "Container failed to start"
- `docker ps` - View all running containers. This is useful to check if your containers
are running properly.

