# Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License")
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

# Run host-side script first in "vagrant up"-command
if ARGV.any? { |arg| arg == "up" }
  puts "Building Docker images before vagrant up..."
  system("./scripts/build_docker_images.sh") or abort("build_docker_images.sh failed, stopping!")
end

Vagrant.configure("2") do |config|
  # There is no official debian 13 image for vagrant, because of the license-change
  # Used this alternative images instead: https://github.com/alchemy-solutions/vagrant-cloud-images
  config.vm.box = "cloud-image/debian-13"

  nodes = {
    "server"  => "192.168.56.10",
    "agent1"  => "192.168.56.11",
    "agent2"  => "192.168.56.12"
  }

  nodes.each do |name, ip|
    config.vm.define name do |node|
      # Add three extra disks of 5G each in addition to the root disk for the onsen
      node.vm.provider :libvirt do |libvirt|
        libvirt.storage :file, size: "5G", name: "ainari-#{name}-vdb.qcow2"
      end

      node.vm.hostname = name
      node.vm.network "private_network", ip: ip
      node.vm.provider "libvirt" do |vb|
        vb.memory = 4096
        vb.cpus = 4
        vb.storage :file, size: 10
      end
    end
  end

  # Run provision.yml for the 'ainari-test' group
  config.vm.provision "ansible" do |ansible|
    ansible.playbook = "testing/vagrant/playbook.yaml"
    ansible.become = true

    ansible.extra_vars = {
      ansible_python_interpreter: "/usr/bin/python3"
    }
    # ansible.verbose = "vvv"
    ansible.groups = {
      "k3s_server" => ["server"],
      "k3s_agents" => ["agent1", "agent2"]
    }
  end
end
