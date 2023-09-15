# authmanager
A simple authentication manager that can be used to manage authorization tokens for the scraper's WebReg API.

## Features
Recall that the `webreg` binary includes a web server that allows external applications to make requests to various WebReg
endpoints; for example, external applications can make requests to the scraper's web server (which will forward that request
to WebReg) to get information like course information, prerequisites, and so on. Additionally, external applications can make
requests to said web server to _perform_ actions like enrolling in classes, dropping classes, and so on.

Now, the web server makes use of _your_ UCSD WebReg session to perform some of these requests. If you intend on making the
web server public, you may not feel comfortable leaving these endpoints exposed for the world to see. Therefore, the `webreg`
binary includes an optional `auth` feature that implements basic authorization measures. By no means is this a "secure" system,
but it should be a decent deterrent against those who might (for some reason) want to use your web server (and thus your account)
in a malicious manner.

The `auth` feature makes use of a bearer token to ensure that people are allowed to make requests through your web server. To 
manage bearer tokens for the server, you can use this binary.

## API Key Structure
Each API key is of the form
```
prefix#token
```
The `prefix` and `token` are both just UUIDs. Generally, the `prefix` is used to refer to an API key. 


## Setup
Below are instructions on how you can run this project. Please ensure that you've set the `webregautoin` script up,
and that it's running, before continuing. Additionally, you'll want to make sure you have the `webreg` executable
running.

### Using Pre-Compiled Executable

1. Download the [relevant executable here](https://github.com/ewang2002/webreg_scraper/releases). The executables will
   be in an archive whose name is of the form `authmanager-[target]`. Here,
    - `[target]` is the target operating system that you can run the executable for. At this time, we have
        - `x86_64-apple-darwin`: MacOS
        - `x86_64-pc-windows-msvc`: Windows
        - `x86_64-unknown-linux-gnu`: Linux
2. Extract the executable from the archive.
3. Ensure that the executable is located in the same directory as the `webreg` executable.

### Self-Compiling Executable
If you're interested in self-compiling, follow the instructions below.

1. You'll need to install [Rust](https://www.rust-lang.org/tools/install). If you're on Ubuntu (or a similar system),
   you'll need to install the following system dependencies:
   ```
   sudo apt-get update
   sudo apt install build-essential
   sudo apt install pkg-config
   ```

2. Clone, or download, this repository.
3. You can now compile the project using the command,
   ```
   cargo build --release --bin authmanager
   ```
4. You should find the `authmanager` executable in the `/target/release` directory.

## Using the Executable
Below are some commands that you'll be able to use.

### Create API Key
To create an API key that can be used by the server, use the command:
```
./authmanager create [--desc <desc>]
```

| Example | Meaning |
| ------- | ------- |
| `./authmanager create` | Creates an API key without any additional description. |
| `./authmanager create --desc "ruby is bad"` | Creates an API key whose description is `ruby is bad` |

### Edit API Key Description
To edit the description of an API key, use the command:
```
./authmanager editDesc --prefix <prefix> [--desc <desc>]
```

| Example | Meaning |
| ------- | ------- |
| `./authmanager editDesc --prefix myprefix` | Sets the description of an API key whose prefix is `myprefix` to nothing. |
| `./authmanager editDesc --prefix myprefix --desc "ruby is bad"` | Sets the description of an API key whose prefix is `myprefix` to `ruby is bad` |

### Delete API Key
To delete an API key, use the command:
```
./authmanager delete --prefix <prefix>
```

| Example | Meaning |
| ------- | ------- |
| `./authmanager delete --prefix myprefix` | Deletes the API key whose prefix is `myprefix`. |

### Check API Key
To check the status of an existing API key, use the command:
```
./authmanager check --prefix <prefix> --token <token>
```

| Example | Meaning |
| ------- | ------- |
| `./authmanager check --prefix myprefix --token mytoken` | Checks if the API key whose prefix is `myprefix` and token is `mytoken` is a valid key. |

### Show All Keys
To show all currently registered keys, use the command:
```
./authmanager showAll [--showToken true|false]
```

| Example | Meaning |
| ------- | ------- |
| `./authmanager showAll` | Shows all API keys (omitting the token). |
| `./authmanager showAll --showToken false` | Shows all API keys (omitting the token). |
| `./authmanager showAll --showToken true` | Shows all API keys (including the token). |
