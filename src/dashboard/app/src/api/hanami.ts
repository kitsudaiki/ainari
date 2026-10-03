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
    FilterDirection,
    FilterIpRangeReq,
    FilterPortReq,
    FloatingIpAttachReq,
    FloatingIpBasicResp,
    FloatingIpCreateReq,
    FloatingIpResp,
    HostResp,
    NetworkBasicResp,
    NetworkFilterResp,
    NetworkCreateReq,
    NetworkResp,
    ProxyBasicResp,
    ProxyResp,
    SakuraHostBasicResp,
    VirtualMachineBasicResp,
    VirtualMachineCreateReq,
    VirtualMachineResp,
    VmTypeBasicResp,
    VmTypeCreateReq,
    VmTypeResp,
    VmTypeUpdateReq,
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
// vm-type
//=============================================================================

/** `GET /v1alpha/vm_type` */
export async function listVmTypes(): Promise<VmTypeBasicResp[]> {
    const resp = await hanamiClient().get("/v1alpha/vm_type");
    return resp.data.vm_types;
}

/** `GET /v1alpha/vm_type/{vm_type_uuid}` */
export async function getVmType(uuid: string): Promise<VmTypeResp> {
    const resp = await hanamiClient().get(`/v1alpha/vm_type/${uuid}`);
    return resp.data;
}

/** `POST /v1alpha/vm_type/admin` */
export async function createVmType(body: VmTypeCreateReq): Promise<VmTypeResp> {
    const resp = await hanamiClient().post("/v1alpha/vm_type/admin", body);
    return resp.data;
}

/**
 * `PUT /v1alpha/vm_type/{vm_type_uuid}/admin`
 *
 * Only the values, which are set in the body, are changed.
 */
export async function updateVmType(
    uuid: string,
    body: VmTypeUpdateReq,
): Promise<VmTypeResp> {
    const resp = await hanamiClient().put(`/v1alpha/vm_type/${uuid}/admin`, body);
    return resp.data;
}

/** `DELETE /v1alpha/vm_type/{vm_type_uuid}/admin` */
export async function deleteVmType(uuid: string): Promise<void> {
    await hanamiClient().delete(`/v1alpha/vm_type/${uuid}/admin`);
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

/** `PUT /v1alpha/floating_ip/{floating_ip_uuid}/attach` */
export async function attachFloatingIp(
    uuid: string,
    body: FloatingIpAttachReq,
): Promise<FloatingIpResp> {
    const resp = await hanamiClient().put(
        `/v1alpha/floating_ip/${uuid}/attach`,
        body,
    );
    return resp.data;
}

/** `PUT /v1alpha/floating_ip/{floating_ip_uuid}/detach` */
export async function detachFloatingIp(uuid: string): Promise<FloatingIpResp> {
    const resp = await hanamiClient().put(`/v1alpha/floating_ip/${uuid}/detach`);
    return resp.data;
}

/** `DELETE /v1alpha/floating_ip/{floating_ip_uuid}` */
export async function deleteFloatingIp(uuid: string): Promise<void> {
    await hanamiClient().delete(`/v1alpha/floating_ip/${uuid}`);
}

//=============================================================================
// network-filter
//=============================================================================

/** Path of the filter of one direction of a virtual machine. */
function networkFilterPath(
    virtualMachineUuid: string,
    direction: FilterDirection,
): string {
    return `/v1alpha/network_filter/${virtualMachineUuid}/${direction}`;
}

/**
 * `GET /v1alpha/network_filter`
 *
 * Directions of virtual machines without a filter are unrestricted and not
 * listed.
 */
export async function listNetworkFilters(): Promise<NetworkFilterResp[]> {
    const resp = await hanamiClient().get("/v1alpha/network_filter");
    return resp.data.network_filters;
}

/** `GET /v1alpha/network_filter/{virtual_machine_uuid}/{direction}` */
export async function getNetworkFilter(
    virtualMachineUuid: string,
    direction: FilterDirection,
): Promise<NetworkFilterResp> {
    const resp = await hanamiClient().get(
        networkFilterPath(virtualMachineUuid, direction),
    );
    return resp.data;
}

/**
 * `DELETE /v1alpha/network_filter/{virtual_machine_uuid}/{direction}`
 *
 * Removes all entries of the filter, which allows all traffic of this direction
 * again.
 */
export async function deleteNetworkFilter(
    virtualMachineUuid: string,
    direction: FilterDirection,
): Promise<void> {
    await hanamiClient().delete(networkFilterPath(virtualMachineUuid, direction));
}

/** `POST /v1alpha/network_filter/{virtual_machine_uuid}/{direction}/ip_range` */
export async function addNetworkFilterIpRanges(
    virtualMachineUuid: string,
    direction: FilterDirection,
    body: FilterIpRangeReq,
): Promise<NetworkFilterResp> {
    const resp = await hanamiClient().post(
        `${networkFilterPath(virtualMachineUuid, direction)}/ip_range`,
        body,
    );
    return resp.data;
}

/** `DELETE /v1alpha/network_filter/{virtual_machine_uuid}/{direction}/ip_range` */
export async function deleteNetworkFilterIpRanges(
    virtualMachineUuid: string,
    direction: FilterDirection,
    body: FilterIpRangeReq,
): Promise<NetworkFilterResp> {
    const resp = await hanamiClient().delete(
        `${networkFilterPath(virtualMachineUuid, direction)}/ip_range`,
        { data: body },
    );
    return resp.data;
}

/** `POST /v1alpha/network_filter/{virtual_machine_uuid}/{direction}/port` */
export async function addNetworkFilterPorts(
    virtualMachineUuid: string,
    direction: FilterDirection,
    body: FilterPortReq,
): Promise<NetworkFilterResp> {
    const resp = await hanamiClient().post(
        `${networkFilterPath(virtualMachineUuid, direction)}/port`,
        body,
    );
    return resp.data;
}

/** `DELETE /v1alpha/network_filter/{virtual_machine_uuid}/{direction}/port` */
export async function deleteNetworkFilterPorts(
    virtualMachineUuid: string,
    direction: FilterDirection,
    body: FilterPortReq,
): Promise<NetworkFilterResp> {
    const resp = await hanamiClient().delete(
        `${networkFilterPath(virtualMachineUuid, direction)}/port`,
        { data: body },
    );
    return resp.data;
}

//=============================================================================
// sakura-host
//=============================================================================

/** `GET /v1alpha/host/admin` */
export async function listSakuraHosts(): Promise<SakuraHostBasicResp[]> {
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

//=============================================================================
// proxy
//=============================================================================

/**
 * `GET /v1alpha/proxy`
 *
 * Lists the proxy-entries of the user. Every virtual-machine gets one entry, which
 * maps a port on the torii at the edge to the sakura of that virtual-machine.
 *
 * The api of the torii is only reachable within the cluster, so the hanami provides
 * this read-only view on the entries. They are created and deleted by the hanami
 * together with the virtual-machine itself.
 */
export async function listProxies(): Promise<ProxyBasicResp[]> {
    const resp = await hanamiClient().get("/v1alpha/proxy");
    // the field is named `proxys` in `ProxyListResp` of the backend
    return resp.data.proxys;
}

/** `GET /v1alpha/proxy/{proxy_uuid}` */
export async function getProxy(uuid: string): Promise<ProxyResp> {
    const resp = await hanamiClient().get(`/v1alpha/proxy/${uuid}`);
    return resp.data;
}
