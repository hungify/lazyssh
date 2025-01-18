#!/bin/bash
set -e

RELEASE="v1.0.0"
OS="$(uname -s)"

case "${OS}" in
MINGW* | Win*) OS="Windows" ;;
esac

if [ -d "$HOME/.lazyssh" ]; then
	INSTALL_DIR="$HOME/.lazyssh"
elif [ -n "$XDG_DATA_HOME" ]; then
	INSTALL_DIR="$XDG_DATA_HOME/lazyssh"
elif [ "$OS" = "Darwin" ]; then
	INSTALL_DIR="$HOME/.lazyssh"
else
	INSTALL_DIR="$HOME/.local/share/lazyssh"
fi

set_filename() {
	if [ "$OS" = "Linux" ]; then
		# Based on https://stackoverflow.com/a/45125525
		case "$(uname -m)" in
		arm | armv7*)
			FILENAME="lazyssh"
			;;
		aarch* | armv8*)
			FILENAME="lazyssh"
			;;
		*)
			FILENAME="lazyssh"
			;;
		esac
	elif [ "$OS" = "Darwin" ] && [ "$FORCE_INSTALL" = "true" ]; then
		FILENAME="lazyssh"
	elif [ "$OS" = "Darwin" ]; then
		FILENAME="lazyssh"
	elif [ "$OS" = "Windows" ]; then
		FILENAME="lazyssh"
		echo "Downloading the latest fnm binary from GitHub..."
	else
		echo "OS $OS is not supported."
		echo "If you think that's a bug - please file an issue to https://github.com/hungify/lazyssh/issues"
		exit 1
	fi
}

download_lazyssh() {
	echo $FILENAME
	if [ "$RELEASE" = "latest" ]; then
		URL="https://github.com/hungify/lazyssh/releases/latest/download/$FILENAME.zip"
	else
		URL="https://github.com/hungify/lazyssh/releases/download/$RELEASE/$FILENAME.zip"
	fi

	DOWNLOAD_DIR=$(mktemp -d)

	echo "Downloading $URL..."

	mkdir -p "$INSTALL_DIR" &>/dev/null

	if ! curl --progress-bar --fail -L "$URL" -o "$DOWNLOAD_DIR/$FILENAME.zip"; then
		echo "Download failed.  Check that the release/filename are correct."
		exit 1
	fi

	unzip -q "$DOWNLOAD_DIR/$FILENAME.zip" -d "$DOWNLOAD_DIR"

	if [ -f "$DOWNLOAD_DIR/lazyssh" ]; then
		mv "$DOWNLOAD_DIR/lazyssh" "$INSTALL_DIR/lazyssh"
	else
		mv "$DOWNLOAD_DIR/$FILENAME/lazyssh" "$INSTALL_DIR/lazyssh"
	fi

	chmod u+x "$INSTALL_DIR/lazyssh"
}

check_dependencies() {
	echo "Checking dependencies for the installation script..."

	echo -n "Checking availability of curl... "
	if hash curl 2>/dev/null; then
		echo "OK!"
	else
		echo "Missing!"
		SHOULD_EXIT="true"
	fi

	echo -n "Checking availability of unzip... "
	if hash unzip 2>/dev/null; then
		echo "OK!"
	else
		echo "Missing!"
		SHOULD_EXIT="true"
	fi

	if [ "$SHOULD_EXIT" = "true" ]; then
		echo "Not installing fnm due to missing dependencies."
		exit 1
	fi
}

ensure_containing_dir_exists() {
	local CONTAINING_DIR
	CONTAINING_DIR="$(dirname "$1")"
	if [ ! -d "$CONTAINING_DIR" ]; then
		echo " >> Creating directory $CONTAINING_DIR"
		mkdir -p "$CONTAINING_DIR"
	fi
}

setup_shell() {
	CURRENT_SHELL="$(basename "$SHELL")"

	if [ "$CURRENT_SHELL" = "zsh" ]; then
		CONF_FILE=${ZDOTDIR:-$HOME}/.zshrc
		ensure_containing_dir_exists "$CONF_FILE"
		echo "Installing for Zsh. Appending the following to $CONF_FILE:"
		{
			echo ''
			echo '# lazyssh'
			echo 'FNM_PATH="'"$INSTALL_DIR"'"'
			echo 'if [ -d "$FNM_PATH" ]; then'
			echo '  export PATH="'$INSTALL_DIR':$PATH"'
			echo 'fi'
		} | tee -a "$CONF_FILE"

	elif [ "$CURRENT_SHELL" = "fish" ]; then
		CONF_FILE=$HOME/.config/fish/conf.d/fnm.fish
		ensure_containing_dir_exists "$CONF_FILE"
		echo "Installing for Fish. Appending the following to $CONF_FILE:"
		{
			echo ''
			echo '# lazyssh'
			echo 'set LAZYSSH_PATH "'"$INSTALL_DIR"'"'
			echo 'if [ -d "$LAZYSSH_PATH" ]'
			echo '  set PATH "$LAZYSSH_PATH" $PATH'
			echo 'end'
		} | tee -a "$CONF_FILE"

	elif [ "$CURRENT_SHELL" = "bash" ]; then
		if [ "$OS" = "Darwin" ]; then
			CONF_FILE=$HOME/.profile
		else
			CONF_FILE=$HOME/.bashrc
		fi
		ensure_containing_dir_exists "$CONF_FILE"
		echo "Installing for Bash. Appending the following to $CONF_FILE:"
		{
			echo ''
			echo '# lazyssh'
			echo 'LAZYSSH_PATH="'"$INSTALL_DIR"'"'
			echo 'if [ -d "$LAZYSSH_PATH" ]; then'
			echo '  export PATH="$LAZYSSH_PATH:$PATH"'
			echo 'fi'
		} | tee -a "$CONF_FILE"

	else
		echo "Could not infer shell type. Please set up manually."
		exit 1
	fi

	echo ""
	echo "In order to apply the changes, open a new terminal or run the following command:"
	echo ""
	echo "  source $CONF_FILE"
}

set_filename
check_dependencies
download_lazyssh
setup_shell
