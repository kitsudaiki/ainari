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

// TypeScript mirror of the response- and request-structs of the rust-crate
// `ainari_api_structs`. Whenever a struct changes over there, it has to be
// changed here as well.

//=============================================================================
// common
//=============================================================================

/** Mirror of `common_structs::Count`. */
export interface Count {
    number_of_items: number;
}

/** Mirror of `common_structs::VersionResp`. */
export interface VersionResp {
    version: string;
    commit_hash: string;
    timestamp: string;
}

//=============================================================================
// endpoints
//=============================================================================

/** Mirror of `endpoints_structs::EndpointField`. */
export interface EndpointField {
    public_address: string;
    internal_address: string;
}

/** Mirror of `endpoints_structs::EndpontsResp`. */
export interface EndpointsResp {
    hanami: EndpointField;
    ryokan: EndpointField;
    torii: EndpointField;
    omamori: EndpointField;
    izakaya: EndpointField;
}

//=============================================================================
// auth
//=============================================================================

/** Mirror of `auth_structs::UserTokenResp`. */
export interface UserTokenResp {
    access_token: string;
    token_type: string;
    expires: number;
}

/** Mirror of `auth_structs::TokenRenewReq`. */
export interface TokenRenewReq {
    /** if not set, the new token is created for the project of the current token */
    project_id?: string;
}

/** Mirror of `auth_structs::PassphraseChangeReq`. */
export interface PassphraseChangeReq {
    old_passphrase: string;
    new_passphrase: string;
}

/** Mirror of `auth_structs::PassphraseChangeAdminReq`. */
export interface PassphraseChangeAdminReq {
    user_id: string;
    new_passphrase: string;
}

//=============================================================================
// user / project / quota (miko)
//=============================================================================

/** Mirror of `user_structs::UserBasicResp`. */
export interface UserBasicResp {
    id: string;
    name: string;
    is_admin: string;
}

/** Mirror of `user_structs::UserResp`. */
export interface UserResp extends UserBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `user_structs::UserCreateReq`. */
export interface UserCreateReq {
    id: string;
    name: string;
    passphrase: string;
    is_admin: string;
}

/** Mirror of `user_structs::UserSetProjectRoleReq`. */
export interface UserSetProjectRoleReq {
    project_id: string;
    project_role: ProjectRole;
}

/** Mirror of `user_structs::UserSetProjectRoleResp`. */
export interface UserSetProjectRoleResp {
    user_id: string;
    project_id: string;
    project_role: ProjectRole;
}

/** Mirror of `project_structs::ProjectBasicResp`. */
export interface ProjectBasicResp {
    id: string;
    name: string;
}

/** Mirror of `project_structs::ProjectResp`. */
export interface ProjectResp extends ProjectBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `project_structs::ProjectCreateReq`. */
export interface ProjectCreateReq {
    id: string;
    name: string;
}

/** Mirror of `enums::ProjectRole`. */
export type ProjectRole = "admin" | "member" | "observer";

/** Mirror of `project_structs::ProjectInvitedResp`. */
export interface ProjectInvitedResp {
    project_id: string;
    project_role: ProjectRole;
}

/** Mirror of `project_structs::ProjectAddUserReq`. */
export interface ProjectAddUserReq {
    user_id: string;
    project_role: ProjectRole;
}

/** Mirror of `project_structs::ProjectAddUserResp`. */
export interface ProjectAddUserResp {
    user_id: string;
    project_id: string;
    project_role: ProjectRole;
}

/** Mirror of `project_structs::ProjectRemoveUserReq`. */
export interface ProjectRemoveUserReq {
    user_id: string;
}

/** Mirror of `project_structs::ProjectMemberResp`. */
export interface ProjectMemberResp {
    user_id: string;
    project_role: ProjectRole;
}

/** Mirror of `quota_structs::QuotaSetReq`. */
export interface QuotaSetReq {
    max_virtual_machine: number;
    max_image: number;
    max_secret: number;
    max_network: number;
    max_floating_ip: number;
}

/** Mirror of `quota_structs::QuotaBasicResp`. */
export interface QuotaBasicResp extends QuotaSetReq {
    project_id: string;
}

/** Mirror of `quota_structs::QuotaResp`. */
export interface QuotaResp extends QuotaBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// virtual-machine (hanami)
//=============================================================================

/** Mirror of `virtual_machine_structs::VirtualMachineCreateReq`. */
export interface VirtualMachineCreateReq {
    name: string;
    /** vm-type, which defines the number of cores and the memory */
    vm_type_uuid: string;
    /** disk-size in GiB */
    disk_size: number;
    network_uuid: string;
}

/** Mirror of `virtual_machine_structs::VirtualMachineBasicResp`. */
export interface VirtualMachineBasicResp {
    uuid: string;
    name: string;
    proxy_port: number;
    number_of_cores: number;
    /** memory in MiB */
    memory_size: number;
    /** disk-size in GiB */
    disk_size: number;
}

/** Allowed values of `virtual_machine_structs::VirtualMachineResp::vm_state`. */
export type VirtualMachineState =
    | "RESERVED"
    | "CREATED"
    | "RUNNING"
    | "STOPPED"
    | "RESTORING"
    | "ERROR";

/** Mirror of `virtual_machine_structs::VirtualMachineResp`. */
export interface VirtualMachineResp {
    uuid: string;
    name: string;
    vm_state: VirtualMachineState;
    number_of_cores: number;
    /** memory in MiB */
    memory_size: number;
    /** disk-size in GiB */
    disk_size: number;
    image_uuid: string;
    network_uuid: string;
    internal_ip: string;
    torii_port: number;
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// vm-type (hanami)
//=============================================================================

/** Mirror of `vm_type_structs::VmTypeCreateReq`. */
export interface VmTypeCreateReq {
    name: string;
    number_of_cores: number;
    /** memory in MiB */
    amount_of_memory: number;
}

/**
 * Mirror of `vm_type_structs::VmTypeUpdateReq`.
 *
 * Only the values, which are set, are updated.
 */
export type VmTypeUpdateReq = Partial<VmTypeCreateReq>;

/** Mirror of `vm_type_structs::VmTypeBasicResp`. */
export interface VmTypeBasicResp extends VmTypeCreateReq {
    uuid: string;
}

/** Mirror of `vm_type_structs::VmTypeResp`. */
export interface VmTypeResp extends VmTypeBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// network / floating-ip (hanami)
//=============================================================================

/** Mirror of `network_structs::NetworkCreateReq`. */
export interface NetworkCreateReq {
    name: string;
    subnet: string;
    /** Don't encrypt the traffic between the virtual machines on different hosts. */
    disable_encryption?: boolean;
}

/** Mirror of `network_structs::NetworkBasicResp`. */
export interface NetworkBasicResp {
    uuid: string;
    name: string;
    subnet: string;
    disable_encryption: boolean;
}

/** Mirror of `network_structs::NetworkResp`. */
export interface NetworkResp extends NetworkBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `floating_ip_structs::FloatingIpCreateReq`. */
export interface FloatingIpCreateReq {
    name: string;
    /** If left out, the backend selects a free address. */
    floating_ip?: string;
    /** If set, the new floating ip is directly attached to this virtual machine. */
    virtual_machine_uuid?: string;
}

/** Mirror of `floating_ip_structs::FloatingIpAttachReq`. */
export interface FloatingIpAttachReq {
    virtual_machine_uuid: string;
}

/** Mirror of `floating_ip_structs::FloatingIpBasicResp`. */
export interface FloatingIpBasicResp {
    uuid: string;
    /** `null`, as long as the floating ip is not attached to a virtual machine. */
    network_uuid: string | null;
    floating_ip: string;
    /** `null`, as long as the floating ip is not attached to a virtual machine. */
    internal_ip: string | null;
}

/** Mirror of `floating_ip_structs::FloatingIpResp`. */
export interface FloatingIpResp extends FloatingIpBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// network-filter
//=============================================================================

/**
 * Mirror of `network_filter_structs::FilterDirection`. `ingress` filters the
 * traffic towards a virtual machine by its source, `egress` the traffic of the
 * virtual machine by its destination.
 */
export type FilterDirection = "ingress" | "egress";

/** Mirror of `network_filter_structs::FilterIpRangeReq`. */
export interface FilterIpRangeReq {
    /** Single addresses, subnets in CIDR notation or ranges `first-last`. */
    ranges: string[];
}

/** Mirror of `network_filter_structs::FilterPortReq`. */
export interface FilterPortReq {
    /** Single ports or ranges `first-last`. */
    ports: string[];
}

/** Mirror of `network_filter_structs::NetworkFilterResp`. */
export interface NetworkFilterResp {
    uuid: string;
    virtual_machine_uuid: string;
    direction: FilterDirection;
    /** Include-list of the ip-ranges in their canonical notation. */
    ip_ranges: string[];
    /** Include-list of the ports in their canonical notation. */
    ports: string[];
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// image (ryokan)
//=============================================================================

/** Image-types accepted by the upload-endpoint of the ryokan. */
export type ImageType = "disk";

/** Mirror of `image_structs::ImageBasicResp`. */
export interface ImageBasicResp {
    uuid: string;
    name: string;
    /** True, if the image is a snapshot of the root-disk of a virtual-machine. */
    is_snapshot: boolean;
}

/** Mirror of `image_structs::ImageResp`. */
export interface ImageResp {
    uuid: string;
    name: string;
    /** True, if the image is a snapshot of the root-disk of a virtual-machine. */
    is_snapshot: boolean;
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// secret / public-key (omamori)
//=============================================================================

/** Mirror of `secret_structs::SecretBasicResp`. */
export interface SecretBasicResp {
    uuid: string;
    name: string;
}

/** Mirror of `secret_structs::SecretResp`. */
export interface SecretResp extends SecretBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `secret_structs::SecretWithPayloadResp`. */
export interface SecretWithPayloadResp {
    secret_payload: string;
}

/** Mirror of `public_key_structs::PublicKeyBasicResp`. */
export interface PublicKeyBasicResp {
    uuid: string;
    name: string;
    fingerprint: string;
}

/** Mirror of `public_key_structs::PublicKeyResp`. */
export interface PublicKeyResp extends PublicKeyBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// host (hanami + ryokan)
//=============================================================================

/** Mirror of `host_structs::HostBasicResp`. */
export interface HostBasicResp {
    uuid: string;
    name: string;
    host_address: string;
}

/**
 * Mirror of `host_structs::SakuraHostBasicResp`. The memory is given in MiB and the
 * disk-space in GiB.
 */
export interface SakuraHostBasicResp extends HostBasicResp {
    number_of_cores: number;
    used_number_of_cores: number;
    memory_size: number;
    amount_of_used_memory: number;
    disk_space: number;
    amount_of_used_disk_space: number;
}

/** Mirror of `host_structs::HostResp`. */
export interface HostResp extends HostBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

//=============================================================================
// task (sakura)
//=============================================================================

/** Mirror of `task_structs::TaskType`. */
export type TaskType =
    | "VirtualMachineCreate"
    | "VirtualMachineDelete"
    | "SnapshotSave"
    | "SnapshotRestore"
    | "VirtualMachineStart"
    | "VirtualMachineStop"
    | "VirtualMachineReboot";

/** Mirror of `task_structs::TaskState`. */
export type TaskState =
    | "Created"
    | "Queued"
    | "Active"
    | "Aborted"
    | "Finished"
    | "Error";

/** Mirror of `task_structs::TaskBasicResp`. */
export interface TaskBasicResp {
    uuid: string;
    description: string;
    task_type: TaskType;
    state: TaskState;
    queued_at: string | null;
    started_at: string | null;
    finished_at: string | null;
}

/** Mirror of `task_structs::TaskResp`. */
export interface TaskResp extends TaskBasicResp {
    messages: string[];
    created_by: string;
}

/** Mirror of `task_structs::VirtualMachineCreateTaskReq`. */
export interface VirtualMachineCreateTaskReq {
    vm_uuid: string;
    image_uuid: string;
    public_key_uuid: string;
}

/** Mirror of `task_structs::TaskSnapshotSaveReq`. */
export interface TaskSnapshotSaveReq {
    name: string;
}

/** Mirror of `task_structs::TaskSnapshotRestoreReq`. */
export interface TaskSnapshotRestoreReq {
    /** Image, which must be a snapshot. */
    image_uuid: string;
}
