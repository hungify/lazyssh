<h1 align="center">
 lazyssh
</h1>

`lazyssh` is a terminal-based SSH client that allows you to easily manage your SSH files. It provides a user-friendly interface to create, delete, and manage SSH keys, as well as interact with the SSH agent.

## Features

- **Create SSH Keys**: Generate new SSH keys with different types and bit lengths.
- **Delete SSH Keys**: Safely delete SSH keys and move them to the trash.
- **Manage SSH Agent**: Add or remove SSH keys from the SSH agent.
- **Copy SSH Public Keys**: Copy SSH public keys to the clipboard for easy sharing.
- **View SSH Key Content**: Display the content of SSH keys directly in the terminal.
- **Command Log**: Keep track of executed commands and their results.

## Key Bindings

- `n`: Create a new SSH key
- `a`: Add a SSH key to the agent
- `d`: Delete a SSH key
- `c`: Copy a SSH public key to the clipboard
- `r`: Remove a SSH key from the agent
- `?`: Show key bindings
- `q`: Quit the application

## Installation

To install `lazyssh`, run the following command:

```sh
curl -fsSL https://raw.githubusercontent.com/hungify/lazyssh/main/install.sh | bash
```

## Usage

Run the application:

```sh
lazyssh
```

## License

This project is licensed under the MIT license. See the [LICENSE](./LICENSE) file for more details.
