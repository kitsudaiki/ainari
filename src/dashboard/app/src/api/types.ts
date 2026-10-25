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

/** Mirror of `project_structs::ProjectLock`. */
export interface ProjectLock {
    /** job, which has locked the project, the nil-UUID for a lock of an admin */
    locked_by_job: string;
    lock_reason: string;
    locked_at: string;
}

/** Mirror of `project_structs::ProjectLockAdminReq`. */
export interface ProjectLockAdminReq {
    reason: string;
}

/** Mirror of `project_structs::ProjectLockResp`. */
export interface ProjectLockResp {
    project_id: string;
    lock: ProjectLock | null;
}

/** Mirror of `project_structs::ProjectResp`. */
export interface ProjectResp extends ProjectBasicResp {
    /** lock of the project during a backup or restore, null if the project is not locked */
    lock?: ProjectLock | null;
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
    /** place the virtual machine on a host, which is isolated for the project */
    isolated_host?: boolean;
}

/** Mirror of `virtual_machine_structs::VirtualMachineBasicResp`. */
export interface VirtualMachineBasicResp {
    uuid: string;
    name: string;
    /** address of the proxy, over which the sakura of the virtual-machine is reachable */
    proxy_address: string;
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
    | "MIGRATING"
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
    /**
     * Address of the proxy, over which the sakura of the virtual-machine is
     * reachable, like `https://<vm-uuid>.<domain>:<port>`. It stays the same,
     * when the virtual-machine is moved to another sakura-host.
     */
    torii_address: string;
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `proxy_structs::ProxyBasicResp`. */
export interface ProxyBasicResp {
    uuid: string;
    /** address, under which the clients reach the target over the proxy */
    address: string;
    target_address: string;
    virtual_machine_uuid: string;
}

/** Mirror of `proxy_structs::ProxyResp`. */
export interface ProxyResp extends ProxyBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `migration_structs::VirtualMachineMigrateReq`. */
export interface VirtualMachineMigrateReq {
    /** sakura-host, which runs the virtual machine after the migration */
    target_host_uuid: string;
}

/** Mirror of `migration_structs::VirtualMachineMigrateResp`. */
export interface VirtualMachineMigrateResp {
    virtual_machine_uuid: string;
    source_host_uuid: string;
    target_host_uuid: string;
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

/**
 * Mirror of `image_structs::ImageStatus`. Only a `READY` image can be used for a virtual-machine.
 * `IN_USE` is reserved and not set at the moment.
 */
export type ImageStatus = "UPLOADING" | "ENCRYPTING" | "READY" | "IN_USE";

/** Mirror of `image_structs::ImageBasicResp`. */
export interface ImageBasicResp {
    uuid: string;
    name: string;
    /** Type of the unencrypted file (e.g. "qcow2"). */
    file_type: string;
    /** Size of the unencrypted file in bytes. */
    file_size: number;
    /** State of the file of the image. */
    image_status: ImageStatus;
    /** True, if the image is a snapshot of the root-disk of a virtual-machine. */
    is_snapshot: boolean;
}

/** Mirror of `image_structs::ImageResp`. */
export interface ImageResp {
    uuid: string;
    name: string;
    /** Type of the unencrypted file (e.g. "qcow2"). */
    file_type: string;
    /** Size of the unencrypted file in bytes. */
    file_size: number;
    /** State of the file of the image. */
    image_status: ImageStatus;
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
    /** host is only used by the virtual machines of a single project */
    is_host_isolated: boolean;
}

/** Mirror of `host_structs::HostResp`. */
export interface HostResp extends HostBasicResp {
    created_at: string;
    created_by: string;
    updated_at: string;
    updated_by: string;
}

/** Mirror of `host_structs::HostBasicResp` of an onsen-host of the ryokan. */
export interface OnsenHostBasicResp extends HostBasicResp {
    /** host only stores backups and never normal images or snapshots */
    is_backup_target: boolean;
}

/** Mirror of `host_structs::HostResp` of an onsen-host of the ryokan. */
export interface OnsenHostResp extends HostResp {
    is_backup_target: boolean;
}

/** Mirror of `host_structs::HostBackupTargetUpdateReq`. */
export interface HostBackupTargetUpdateReq {
    is_backup_target: boolean;
}

/** Mirror of `host_structs::HostIsolationUpdateReq`. */
export interface HostIsolationUpdateReq {
    /** host is only used by the virtual machines of a single project */
    is_host_isolated: boolean;
}

/** Mirror of `host_structs::SakuraHostResp`. */
export interface SakuraHostResp extends HostResp {
    /** host is only used by the virtual machines of a single project */
    is_host_isolated: boolean;
    /** project, which the isolated host is bound to */
    project_id: string | null;
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
    /** Existing secret to encrypt the snapshot with. Without it, a new secret is generated. */
    secret_uuid?: string;
}

/** Mirror of `task_structs::TaskSnapshotRestoreReq`. */
export interface TaskSnapshotRestoreReq {
    /** Image, which must be a snapshot. */
    image_uuid: string;
}

//=============================================================================
// error-log / audit-log (omikuji)
//=============================================================================

/** Mirror of `log_structs::ErrorLogResp`. */
export interface ErrorLogResp {
    uuid: string;
    message: string;
    component_name: string;
    host_name: string;
    user_id: string;
    project_id: string;
    created_at: string;
}

/** Mirror of `log_structs::AuditLogResp`. */
export interface AuditLogResp {
    uuid: string;
    request_type: string;
    endpoint: string;
    component_name: string;
    host_name: string;
    user_id: string;
    project_id: string;
    created_at: string;
}

//=============================================================================
// backup / restore (isekai)
//=============================================================================

/** Mirror of `backup_structs::BackupKind`. */
export type BackupKind = "PROJECT" | "VIRTUAL_MACHINE";

/** Mirror of `backup_structs::BackupStatus`. */
export type BackupStatus = "CREATING" | "READY" | "DELETING" | "ERROR";

/** Mirror of `backup_structs::JobType`. */
export type JobType = "BACKUP" | "RESTORE";

/** Mirror of `backup_structs::JobState`. */
export type JobState = "QUEUED" | "ACTIVE" | "FINISHED" | "ERROR";

/** Minimum length of the passphrase of a new backup, see `MIN_BACKUP_PASSPHRASE_LENGTH`. */
export const MIN_BACKUP_PASSPHRASE_LENGTH = 12;

/** Mirror of `backup_structs::ProjectBackupCreateReq`. */
export interface ProjectBackupCreateReq {
    name: string;
    passphrase: string;
    /** shut the running virtual machines down, while their root-disks are saved */
    stop_vms: boolean;
    /** copy the images and snapshots of the project into the backup */
    include_images: boolean;
}

/** Mirror of `backup_structs::VirtualMachineBackupCreateReq`. */
export interface VirtualMachineBackupCreateReq {
    virtual_machine_uuid: string;
    name: string;
    passphrase: string;
    stop_vm: boolean;
}

/** Mirror of `backup_structs::BackupRestoreReq`. */
export interface BackupRestoreReq {
    passphrase: string;
    force: boolean;
    prune: boolean;
    target_project_id?: string;
    start_vms: boolean;
    include_dependencies: boolean;
    dry_run: boolean;
}

/** Mirror of `backup_structs::IsekaiBackupResp`. */
export interface BackupResp {
    uuid: string;
    name: string;
    kind: BackupKind;
    project_id: string;
    virtual_machine_uuid: string | null;
    state: BackupStatus;
    ainari_version: string;
    host_uuid: string | null;
    file_count: number | null;
    total_size: number | null;
    created_at: string;
    created_by: string;
}

/** Mirror of `backup_structs::IdMapEntryResp`. */
export interface IdMapEntryResp {
    resource_type: string;
    old_uuid: string;
    new_uuid: string;
}

/** Mirror of `backup_structs::JobResp`. */
export interface JobResp {
    uuid: string;
    job_type: JobType;
    backup_uuid: string;
    project_id: string;
    state: JobState;
    step: string;
    messages: string[];
    id_map: IdMapEntryResp[];
    created_at: string;
    created_by: string;
    started_at: string | null;
    finished_at: string | null;
}

/** Mirror of `backup_structs::BackupJobResp`. */
export interface BackupJobResp {
    backup: BackupResp;
    job: JobResp;
}

/** Mirror of `backup_structs::PlannedResource`. */
export interface PlannedResource {
    resource_type: string;
    uuid: string;
    name: string;
}

/** Mirror of `backup_structs::QuotaUsageResp`. */
export interface QuotaUsageResp {
    resource_type: string;
    current: number;
    after_restore: number;
    max: number;
}

/** Mirror of `backup_structs::RestorePlanResp`. */
export interface RestorePlanResp {
    target_project_id: string;
    create_project: boolean;
    id_map: IdMapEntryResp[];
    overwritten: PlannedResource[];
    deleted: PlannedResource[];
    quota: QuotaUsageResp[];
    warnings: string[];
}

/** Mirror of `backup_structs::RestoreResp`. */
export interface RestoreResp {
    plan: RestorePlanResp;
    /** empty for a dry-run */
    job: JobResp | null;
}

/** Mirror of `backup_structs::BackupScanEntry`. */
export interface BackupScanEntry {
    uuid: string;
    has_manifest: boolean;
}

/** Mirror of `backup_structs::BackupImportReq`. */
export interface BackupImportReq {
    onsen_host_uuid: string;
    backup_uuid: string;
    passphrase: string;
    name?: string;
}
