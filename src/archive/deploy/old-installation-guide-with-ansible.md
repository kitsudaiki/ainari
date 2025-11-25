## With Ansible

### Requirements

- Only Ansible itself is required to be installed

- Other dependencies like docker will be installed by the ansible playbooks, so it is required that
    playbooks can be executed with sudo permissions

### Environment Variables

Basic parameter have to be set by environemt-variables:

- `ADMIN_USER_ID`

    - Identifier for the new user. It is used for login and internal references to the user.
    - String, which MUST match the regex `[a-zA-Z][a-zA-Z_0-9@]*` with between `4` and `256`
        characters length

- `ADMIN_USER_NAME`

    - Better readable name for the user, which doesn't have to be unique in the system.
    - String, which MUST match the regex `[a-zA-Z][a-zA-Z_0-9 ]*` with between `4` and `256`
        characters length

- `ADMIN_PASSPHRASE`

    - Passphrase for the initial user
    - String, with between `8` and `4096` characters length

- `TOKEN_KEY`

    - Key for the JWT-Tokens
    - String

!!! example

    ```bash
    export ADMIN_USER_ID="admin"
    export ADMIN_USER_NAME="admin"
    export ADMIN_PASSPHRASE="some_passphrase"
    export TOKEN_KEY="random_token_key"
    ```

### Run

Run in the root of the repository:

```bash
cd deploy/ansible/

ansible-playbook --connection=local -i ainari/inventory.yml ainari/deploy.yml
```

The resulting setup will listen on `0.0.0.0` and port `443` and `80`.

### Testing

The playbooks can be tested with `vagrant`

```bash
apt-get install vagrant virtualbox

vagrant plugin install vagrant-env
vagrant plugin install vagrant-vbguest
```

The environment variables also must be set in this case with the initial admin credentials.

Run in the root of the repository:

```bash
cd testing/ansible_deploy

vagrant up
```

It will create a virtualbox-VM with ubuntu 22.04 and automatically deploy Ainari with the
ansible-playbook.
