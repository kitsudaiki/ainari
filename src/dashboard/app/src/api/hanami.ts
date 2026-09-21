// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Endpoints of the hanami, mirroring `src/binaries/hanami/src/api/routes/v1alpha.rs`.

import { hanamiClient } from "./client";
import type {
    FloatingIpBasicResp,
    FloatingIpCreateReq,
    FloatingIpResp,
    HostBasicResp,
    HostResp,
    NetworkBasicResp,
    NetworkCreateReq,
    NetworkResp,
    VirtualMachineBasicResp,
    VirtualMachineCreateReq,
    VirtualMachineResp,
} from "./types";

//=============================================================================
// virtual-machine
//=============================================================================

/**
 * `POST /v1alpha/virtual_machine`
 *
 * Reserves a new virtual-machine on one of the sakura-hosts. Neither the image nor
 * the public-key are deployed here. That is done afterwards by the create-task on
 * the sakura of the reserved virtual-machine, see `sakura.createVirtualMachine`.
 */
export async function reserveVirtualMachine(
    body: VirtualMachineCreateReq,
): Promise<VirtualMachineResp> {
    const resp = await hanamiClient().post("/v1alpha/virtual_machine", body);
    return resp.data;
}

/** `GET /v1alpha/virtual_machine` */
export async function listVirtualMachines(): Promise<VirtualMachineBasicResp[]> {
    const resp = await hanamiClient().get("/v1alpha/virtual_machine");
    return resp.data.virtual_machines;
}

/** `GET /v1alpha/virtual_machine/{virtual_machine_uuid}` */
export async function getVirtualMachine(uuid: string): Promise<VirtualMachineResp> {
    const resp = await hanamiClient().get(`/v1alpha/virtual_machine/${uuid}`);
    return resp.data;
}

/** `DELETE /v1alpha/virtual_machine/{virtual_machine_uuid}` */
export async function deleteVirtualMachine(uuid: string): Promise<void> {
    await hanamiClient().delete(`/v1alpha/virtual_machine/${uuid}`);
}

/** `GET /v1alpha/virtual_machine/count` */
export async function getVirtualMachineCount(): Promise<number> {
    const resp = await hanamiClient().get("/v1alpha/virtual_machine/count");
    return resp.data.number_of_items;
}

//=============================================================================
// network
//=============================================================================

/** `POST /v1alpha/network` */
export async function createNetwork(body: NetworkCreateReq): Promise<NetworkResp> {
    const resp = await hanamiClient().post("/v1alpha/network", body);
    return resp.data;
}

/** `GET /v1alpha/network` */
export async function listNetworks(): Promise<NetworkBasicResp[]> {
    const resp = await hanamiClient().get("/v1alpha/network");
    return resp.data.networks;
}

/** `GET /v1alpha/network/{network_uuid}` */
export async function getNetwork(uuid: string): Promise<NetworkResp> {
    const resp = await hanamiClient().get(`/v1alpha/network/${uuid}`);
    return resp.data;
}

/** `DELETE /v1alpha/network/{network_uuid}` */
export async function deleteNetwork(uuid: string): Promise<void> {
    await hanamiClient().delete(`/v1alpha/network/${uuid}`);
}

//=============================================================================
// floating-ip
//=============================================================================

/** `POST /v1alpha/floating_ip` */
export async function createFloatingIp(
    body: FloatingIpCreateReq,
): Promise<FloatingIpResp> {
    const resp = await hanamiClient().post("/v1alpha/floating_ip", body);
    return resp.data;
}

/** `GET /v1alpha/floating_ip` */
export async function listFloatingIps(): Promise<FloatingIpBasicResp[]> {
    const resp = await hanamiClient().get("/v1alpha/floating_ip");
    return resp.data.floating_ips;
}

/** `GET /v1alpha/floating_ip/{floating_ip_uuid}` */
export async function getFloatingIp(uuid: string): Promise<FloatingIpResp> {
    const resp = await hanamiClient().get(`/v1alpha/floating_ip/${uuid}`);
    return resp.data;
}

/** `DELETE /v1alpha/floating_ip/{floating_ip_uuid}` */
export async function deleteFloatingIp(uuid: string): Promise<void> {
    await hanamiClient().delete(`/v1alpha/floating_ip/${uuid}`);
}

//=============================================================================
// sakura-host
//=============================================================================

/** `GET /v1alpha/host/admin` */
export async function listSakuraHosts(): Promise<HostBasicResp[]> {
    const resp = await hanamiClient().get("/v1alpha/host/admin");
    return resp.data.hosts;
}

/** `GET /v1alpha/host/{host_uuid}/admin` */
export async function getSakuraHost(uuid: string): Promise<HostResp> {
    const resp = await hanamiClient().get(`/v1alpha/host/${uuid}/admin`);
    return resp.data;
}

/** `DELETE /v1alpha/host/{host_uuid}/admin` */
export async function deleteSakuraHost(uuid: string): Promise<void> {
    await hanamiClient().delete(`/v1alpha/host/${uuid}/admin`);
}
