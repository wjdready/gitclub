<template>
  <div class="member-management">
    <div class="section">
      <h3>Owner</h3>
      <div v-if="owner" class="owner-info">
        <div class="user-card">
          <img v-if="owner.avatar_url" :src="owner.avatar_url" class="avatar" />
          <div v-else class="avatar-placeholder">{{ getInitials(owner.username) }}</div>
          <div class="user-details">
            <div class="username">{{ owner.display_name || owner.username }}</div>
            <div class="user-meta">@{{ owner.username }}</div>
          </div>
        </div>
      </div>
      <div v-else class="no-owner">No owner assigned</div>
    </div>

    <div class="section">
      <div class="section-header">
        <h3>Members ({{ members.length }})</h3>
        <button v-if="canManage" class="btn-add" @click="showAddModal = true">Add Member</button>
      </div>

      <div v-if="members.length > 0" class="members-list">
        <div v-for="member in members" :key="member.user_id" class="member-item">
          <div class="user-card">
            <div class="avatar-placeholder">{{ getInitials(member.username) }}</div>
            <div class="user-details">
              <div class="username">{{ member.username }}</div>
              <div class="user-meta">
                {{ member.role }} · Added {{ formatDate(member.created_at) }}
                <span v-if="resourceType === 'group' && member.can_view_subgroups" class="permission-badge">
                  Can view subgroups
                </span>
              </div>
            </div>
          </div>
          <div class="member-actions">
            <button v-if="canManage && resourceType === 'group'" class="btn-edit" @click="editMember(member)">Edit</button>
            <button v-if="canManage" class="btn-remove" @click="removeMember(member.username)">Remove</button>
          </div>
        </div>
      </div>
      <div v-else class="no-members">No members yet</div>
    </div>

    <!-- Add Member Modal -->
    <div v-if="showAddModal" class="modal-overlay" @click.self="showAddModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>Add Member</h3>
          <button class="btn-close" @click="showAddModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>Username</label>
            <input v-model="newMemberUsername" type="text" placeholder="Enter username" />
          </div>
          <div class="form-group">
            <label>Role</label>
            <select v-model="newMemberRole">
              <option value="member">Member</option>
              <option value="admin">Admin</option>
              <option value="reader">Reader</option>
            </select>
          </div>
          <div v-if="resourceType === 'group'" class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="canViewSubgroups" />
              <span>Can view subgroups</span>
            </label>
            <p class="help-text">If enabled, this member can see all subgroups and repositories within them.</p>
          </div>
          <div v-if="addError" class="error-message">{{ addError }}</div>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel" @click="showAddModal = false">Cancel</button>
          <button class="btn-primary" @click="addMember" :disabled="!newMemberUsername">Add</button>
        </div>
      </div>
    </div>

    <!-- Edit Member Modal -->
    <div v-if="showEditModal" class="modal-overlay" @click.self="showEditModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>Edit Member Permissions</h3>
          <button class="btn-close" @click="showEditModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>Username</label>
            <input v-model="editingMember.username" type="text" disabled />
          </div>
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="editingMember.can_view_subgroups" />
              <span>Can view subgroups</span>
            </label>
            <p class="help-text">If enabled, this member can see all subgroups and repositories within them.</p>
          </div>
          <div v-if="editError" class="error-message">{{ editError }}</div>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel" @click="showEditModal = false">Cancel</button>
          <button class="btn-primary" @click="updateMemberPermissions">Save</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  owner: Object,
  members: Array,
  resourcePath: String,
  resourceType: String, // 'group' or 'repo'
  canManage: Boolean,
})

const emit = defineEmits(['member-added', 'member-removed'])

const showAddModal = ref(false)
const showEditModal = ref(false)
const newMemberUsername = ref('')
const newMemberRole = ref('member')
const canViewSubgroups = ref(false)
const addError = ref('')
const editError = ref('')
const editingMember = ref({
  username: '',
  can_view_subgroups: false
})

const getInitials = (name) => {
  if (!name) return '?'
  return name.substring(0, 2).toUpperCase()
}

const formatDate = (dateStr) => {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now - date
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) return 'today'
  if (days === 1) return 'yesterday'
  if (days < 7) return `${days} days ago`
  if (days < 30) return `${Math.floor(days / 7)} weeks ago`
  if (days < 365) return `${Math.floor(days / 30)} months ago`
  return `${Math.floor(days / 365)} years ago`
}

const addMember = async () => {
  addError.value = ''

  try {
    const endpoint = props.resourceType === 'group' ? '/api/group-members' : '/api/repo-members'
    const body = {
      path: props.resourcePath,
      username: newMemberUsername.value,
      role: newMemberRole.value,
    }

    // 只有组成员才有 can_view_subgroups 字段
    if (props.resourceType === 'group') {
      body.can_view_subgroups = canViewSubgroups.value
    }

    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })

    if (!response.ok) {
      const data = await response.json()
      throw new Error(data.error || 'Failed to add member')
    }

    showAddModal.value = false
    newMemberUsername.value = ''
    newMemberRole.value = 'member'
    canViewSubgroups.value = false
    emit('member-added')
  } catch (error) {
    addError.value = error.message
  }
}

const editMember = (member) => {
  editingMember.value = {
    username: member.username,
    can_view_subgroups: member.can_view_subgroups || false
  }
  editError.value = ''
  showEditModal.value = true
}

const updateMemberPermissions = async () => {
  editError.value = ''

  try {
    const response = await fetch('/api/group-members/permissions', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        path: props.resourcePath,
        username: editingMember.value.username,
        can_view_subgroups: editingMember.value.can_view_subgroups,
      }),
    })

    if (!response.ok) {
      const data = await response.json()
      throw new Error(data.error || 'Failed to update permissions')
    }

    showEditModal.value = false
    emit('member-added') // 触发刷新
  } catch (error) {
    editError.value = error.message
  }
}

const removeMember = async (username) => {
  if (!confirm(`Remove ${username} from this ${props.resourceType}?`)) return

  try {
    const endpoint = props.resourceType === 'group' ? '/api/group-members/remove' : '/api/repo-members/remove'
    const response = await fetch(endpoint, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        path: props.resourcePath,
        username,
      }),
    })

    if (!response.ok) {
      const data = await response.json()
      throw new Error(data.error || 'Failed to remove member')
    }

    emit('member-removed')
  } catch (error) {
    alert('Failed to remove member: ' + error.message)
  }
}
</script>

<style scoped>
.member-management {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.section {
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  padding: 16px;
}

.section h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.section-header h3 {
  margin: 0;
}

.owner-info, .no-owner {
  padding: 8px 0;
}

.no-owner, .no-members {
  color: #57606a;
  font-size: 14px;
  padding: 16px;
  text-align: center;
}

.user-card {
  display: flex;
  align-items: center;
  gap: 12px;
}

.avatar, .avatar-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  flex-shrink: 0;
}

.avatar {
  object-fit: cover;
}

.avatar-placeholder {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 14px;
}

.user-details {
  flex: 1;
}

.username {
  font-weight: 600;
  font-size: 14px;
  color: #24292f;
}

.user-meta {
  font-size: 12px;
  color: #57606a;
  margin-top: 2px;
}

.members-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.member-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  border-radius: 6px;
  transition: background 0.2s;
}

.member-item:hover {
  background: #f6f8fa;
}

.member-actions {
  display: flex;
  gap: 8px;
}

.permission-badge {
  display: inline-block;
  background: #ddf4ff;
  color: #0969da;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
  margin-left: 8px;
}

.btn-edit {
  background: transparent;
  color: #0969da;
  border: 1px solid #0969da;
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-edit:hover {
  background: #0969da;
  color: white;
}

.btn-add {
  background: #2da44e;
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-add:hover {
  background: #2c974b;
}

.btn-remove {
  background: transparent;
  color: #cf222e;
  border: 1px solid #cf222e;
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-remove:hover {
  background: #cf222e;
  color: white;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: white;
  border-radius: 8px;
  width: 90%;
  max-width: 500px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #d0d7de;
}

.modal-header h3 {
  margin: 0;
  font-size: 18px;
}

.btn-close {
  background: none;
  border: none;
  font-size: 28px;
  color: #57606a;
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: background 0.2s;
}

.btn-close:hover {
  background: #f6f8fa;
}

.modal-body {
  padding: 20px;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 6px;
  color: #24292f;
}

.form-group input,
.form-group select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  font-size: 14px;
  box-sizing: border-box;
}

.form-group input:focus,
.form-group select:focus {
  outline: none;
  border-color: #0969da;
  box-shadow: 0 0 0 3px rgba(9, 105, 218, 0.1);
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-weight: 600;
  font-size: 14px;
  color: #24292f;
}

.checkbox-label input[type="checkbox"] {
  width: auto;
  cursor: pointer;
}

.help-text {
  margin: 8px 0 0 0;
  font-size: 12px;
  color: #57606a;
  font-weight: normal;
}

.error-message {
  color: #cf222e;
  font-size: 14px;
  margin-top: 8px;
  padding: 8px 12px;
  background: #ffebe9;
  border-radius: 6px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid #d0d7de;
}

.btn-cancel {
  background: transparent;
  color: #24292f;
  border: 1px solid #d0d7de;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-cancel:hover {
  background: #f6f8fa;
}

.btn-primary {
  background: #2da44e;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary:hover:not(:disabled) {
  background: #2c974b;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
