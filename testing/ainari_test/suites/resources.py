# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
The resources, which the virtual machines are built from: public key, image and network. Secrets
are tested here as well, because they live in omamori next to the public keys.
"""

import os
import urllib.request
import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import image
from ainari_sdk import network
from ainari_sdk import public_key
from ainari_sdk import secret

from ainari_test.checks import (check, check_equal, check_in, check_keys, exists,
                                expect_error)
from ainari_test.framework import Suite
from ainari_test.waiting import wait_until

suite = Suite("resources", "public key, image, network and secrets")


@suite.test("generate and upload ssh public key", provides=("public_key",))
def upload_public_key(ctx):
    key_content = ctx.ssh.generate_key()
    name = ctx.name("key")
    result = public_key.upload_public_key(ctx.api, name, key_content)
    key_uuid = result["uuid"]
    ctx.cleanup.add("public_key", key_uuid, name,
                    lambda: public_key.delete_public_key(ctx.api, key_uuid),
                    lambda: exists(public_key.get_public_key, ctx.api, key_uuid))
    ctx.state["public_key"] = key_uuid

    check_equal(result["name"], name, "name of the public key")
    check(result["fingerprint"], "public key has no fingerprint")
    ctx.log(f"public key {key_uuid} ({result['fingerprint']})")


@suite.test("get and list public key", requires=("public_key",))
def get_public_key(ctx):
    key_uuid = ctx.state["public_key"]
    result = public_key.get_public_key(ctx.api, key_uuid)
    check_equal(result["name"], ctx.name("key"), "name of the public key")
    check_in(key_uuid, [entry["uuid"] for entry in public_key.list_public_keys(ctx.api)
                        ["public_keys"]], "uploaded public key in list")


@suite.test("invalid public key is rejected")
def invalid_public_key(ctx):
    expect_error(ainari_exceptions.BadRequestException,
                 public_key.upload_public_key, ctx.api, ctx.name("invalid-key"),
                 "this is not an ssh public key")


@suite.test("upload cloud-image", provides=("image",))
def upload_image(ctx):
    image_path = ctx.config.image_path
    if os.path.exists(image_path) and os.path.getsize(image_path) > 0:
        ctx.log(f"cloud-image already downloaded: {image_path}")
    else:
        ctx.log(f"downloading {ctx.config.image_url} ...")
        urllib.request.urlretrieve(ctx.config.image_url, image_path)
        ctx.log(f"downloaded {os.path.getsize(image_path)} bytes")

    ctx.log("uploading the cloud-image to ryokan (this takes a while) ...")
    name = ctx.name("image")
    result = image.upload_disk_file(ctx.api, name, image_path)
    image_uuid = result["uuid"]
    ctx.cleanup.add("image", image_uuid, name,
                    lambda: image.delete_image(ctx.api, image_uuid),
                    lambda: exists(image.get_image, ctx.api, image_uuid))
    ctx.state["image"] = image_uuid
    ctx.log(f"image {image_uuid}")


@suite.test("get, list and count images", requires=("image",))
def get_image(ctx):
    image_uuid = ctx.state["image"]
    result = image.get_image(ctx.api, image_uuid)
    check_equal(result["name"], ctx.name("image"), "name of the image")
    check_equal(result["is_snapshot"], False, "image is a snapshot")

    images = image.list_images(ctx.api)["images"]
    check_in(image_uuid, [entry["uuid"] for entry in images], "uploaded image in list")
    count = image.get_image_count(ctx.api)
    check_keys(count, ["number_of_items"], "image-count")
    check(count["number_of_items"] >= 1, f"image-count is {count['number_of_items']}")


@suite.test("unknown image is not found")
def unknown_image(ctx):
    expect_error(ainari_exceptions.NotFoundException, image.get_image, ctx.api,
                 str(uuid.uuid4()))


@suite.test("create network", provides=("network",))
def create_network(ctx):
    name = ctx.name("net")
    result = network.create_network(ctx.api, name, ctx.config.network_subnet)
    network_uuid = result["uuid"]

    def delete():
        # the virtual machines are deleted by their sakura-hosts in the background, so the
        # network can still be in use for a moment after they are gone in hanami
        def try_delete():
            try:
                network.delete_network(ctx.api, network_uuid)
            except ainari_exceptions.ConflictException:
                return False
            return True
        wait_until(try_delete, ctx.config.delete_timeout, f"network '{network_uuid}' deleted")

    ctx.cleanup.add("network", network_uuid, name, delete,
                    lambda: exists(network.get_network, ctx.api, network_uuid))
    ctx.state["network"] = network_uuid

    check_equal(result["name"], name, "name of the network")
    check_equal(result["subnet"], ctx.config.network_subnet, "subnet of the network")
    ctx.log(f"network {network_uuid} ({result['subnet']})")


@suite.test("get and list network", requires=("network",))
def get_network(ctx):
    network_uuid = ctx.state["network"]
    result = network.get_network(ctx.api, network_uuid)
    check_equal(result["name"], ctx.name("net"), "name of the network")
    check_equal(result["subnet"], ctx.config.network_subnet, "subnet of the network")
    check_in(network_uuid, [entry["uuid"] for entry in network.list_networks(ctx.api)
                            ["networks"]], "network in list")


@suite.test("unknown network is not found")
def unknown_network(ctx):
    expect_error(ainari_exceptions.NotFoundException, network.get_network, ctx.api,
                 str(uuid.uuid4()))


@suite.test("secret life-cycle")
def secret_lifecycle(ctx):
    payload = f"payload-{uuid.uuid4()}"
    created = secret.create_secret(ctx.api, ctx.name("secret"), payload)
    secret_uuid = created["uuid"]
    ctx.cleanup.add("secret", secret_uuid, ctx.name("secret"),
                    lambda: secret.delete_secret(ctx.api, secret_uuid),
                    lambda: exists(secret.get_secret, ctx.api, secret_uuid))

    check_equal(secret.get_secret(ctx.api, secret_uuid)["name"], ctx.name("secret"),
                "name of the secret")
    check_equal(secret.get_secret_payload(ctx.api, secret_uuid)["secret_payload"], payload,
                "payload of the secret")
    check_in(secret_uuid, [entry["uuid"] for entry in secret.list_secrets(ctx.api)["secrets"]],
             "secret in list")
    check(secret.get_secret_count(ctx.api)["number_of_items"] >= 1, "secret-count is 0")

    generated = secret.generate_secret(ctx.api, ctx.name("generated"))
    generated_uuid = generated["uuid"]
    ctx.cleanup.add("secret", generated_uuid, ctx.name("generated"),
                    lambda: secret.delete_secret(ctx.api, generated_uuid),
                    lambda: exists(secret.get_secret, ctx.api, generated_uuid))
    check(secret.get_secret_payload(ctx.api, generated_uuid)["secret_payload"],
          "generated secret has no payload")

    for entry_uuid in (secret_uuid, generated_uuid):
        secret.delete_secret(ctx.api, entry_uuid)
        ctx.cleanup.discard(entry_uuid)
        check(not exists(secret.get_secret, ctx.api, entry_uuid),
              f"secret '{entry_uuid}' still exists after its deletion")
