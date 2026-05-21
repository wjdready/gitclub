<template>
  <div class="profile-container">
    <div class="profile-header">
      <div class="avatar-section">
        <div class="avatar">
          <img v-if="user?.avatar_url" :src="user.avatar_url" :alt="user.username" />
          <div v-else class="avatar-placeholder">
            {{ getInitials(user?.display_name || user?.username) }}
          </div>
        </div>
        <button v-if="!isEditing" class="btn-edit" @click="startEdit">Edit Profile</button>
      </div>
      <div class="user-info">
        <h1>{{ user?.display_name || user?.username }}</h1>
        <p class="username">@{{ user?.username }}</p>
        <span v-if="user?.is_admin" class="badge admin">Admin</span>
      </div>
    </div>

    <div class="profile-body">
      <div v-if="error" class="error-message">{{ error }}</div>
      <div v-if="success" class="success-message">{{ success }}</div>

      <div v-if="!isEditing" class="info-section">
        <h2>Profile Information</h2>
        <div class="info-grid">
          <div class="info-item">
            <span class="label">Email</span>
            <span class="value">{{ user?.email || 'Not set' }}</span>
          </div>
          <div class="info-item">
            <span class="label">Display Name</span>
            <span class="value">{{ user?.display_name || 'Not set' }}</span>
          </div>
          <div class="info-item">
            <span class="label">Bio</span>
            <span class="value">{{ user?.bio || 'Not set' }}</span>
          </div>
        </div>
      </div>

      <form v-else @submit.prevent="saveProfile" class="edit-form">
        <h2>Edit Profile</h2>

        <div class="form-group">
          <label for="display-name">Display Name</label>
          <input
            id="display-name"
            v-model="editForm.display_name"
            type="text"
            placeholder="Your display name"
          />
        </div>

        <div class="form-group">
          <label for="email">Email</label>
          <input
            id="email"
            v-model="editForm.email"
            type="email"
            placeholder="your.email@example.com"
          />
        </div>

        <div class="form-group">
          <label for="bio">Bio</label>
          <textarea
            id="bio"
            v-model="editForm.bio"
            rows="4"
            placeholder="Tell us about yourself..."
          ></textarea>
        </div>

        <div class="form-group">
          <label for="avatar-url">Avatar URL</label>
          <input
            id="avatar-url"
            v-model="editForm.avatar_url"
            type="url"
            placeholder="https://example.com/avatar.jpg"
          />
        </div>

        <div class="form-actions">
          <button type="submit" class="btn-save" :disabled="saving">
            {{ saving ? 'Saving...' : 'Save Changes' }}
          </button>
          <button type="button" class="btn-cancel" @click="cancelEdit">
            Cancel
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const props = defineProps({
  user: {
    type: Object,
    required: true
  }
})

const emit = defineEmits(['profile-updated'])

const isEditing = ref(false)
const saving = ref(false)
const error = ref('')
const success = ref('')

const editForm = ref({
  display_name: '',
  email: '',
  bio: '',
  avatar_url: ''
})

const getInitials = (name) => {
  if (!name) return '?'
  const parts = name.split(' ')
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase()
  }
  return name.substring(0, 2).toUpperCase()
}

const startEdit = () => {
  editForm.value = {
    display_name: props.user.display_name || '',
    email: props.user.email || '',
    bio: props.user.bio || '',
    avatar_url: props.user.avatar_url || ''
  }
  isEditing.value = true
  error.value = ''
  success.value = ''
}

const cancelEdit = () => {
  isEditing.value = false
  error.value = ''
  success.value = ''
}

const saveProfile = async () => {
  error.value = ''
  success.value = ''
  saving.value = true

  try {
    const response = await fetch('/api/user/profile', {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(editForm.value),
    })

    const data = await response.json()

    if (response.ok && data.success) {
      success.value = 'Profile updated successfully'
      isEditing.value = false
      emit('profile-updated', data.user)
    } else {
      error.value = data.message || 'Failed to update profile'
    }
  } catch (err) {
    error.value = 'Network error. Please try again.'
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.profile-container {
  max-width: 900px;
  margin: 0 auto;
}

.profile-header {
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  padding: 32px;
  margin-bottom: 24px;
  display: flex;
  gap: 32px;
  align-items: flex-start;
}

.avatar-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.avatar {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  overflow: hidden;
  border: 3px solid #d0d7de;
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.avatar-placeholder {
  width: 100%;
  height: 100%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 48px;
  font-weight: 600;
  color: white;
}

.btn-edit {
  background: #0969da;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-edit:hover {
  background: #0860ca;
}

.user-info {
  flex: 1;
}

.user-info h1 {
  font-size: 28px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: #24292f;
}

.username {
  font-size: 18px;
  color: #57606a;
  margin: 0 0 12px 0;
}

.badge {
  display: inline-block;
  background: #0969da;
  color: white;
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.badge.admin {
  background: #8250df;
}

.profile-body {
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  padding: 32px;
}

.error-message {
  background: #fff1f0;
  border: 1px solid #ffa39e;
  color: #cf222e;
  padding: 12px;
  border-radius: 6px;
  font-size: 14px;
  margin-bottom: 20px;
}

.success-message {
  background: #dafbe1;
  border: 1px solid #4ac26b;
  color: #1a7f37;
  padding: 12px;
  border-radius: 6px;
  font-size: 14px;
  margin-bottom: 20px;
}

.info-section h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0 0 24px 0;
  color: #24292f;
}

.info-grid {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.info-item {
  display: flex;
  gap: 16px;
  padding-bottom: 20px;
  border-bottom: 1px solid #d0d7de;
}

.info-item:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.info-item .label {
  font-weight: 600;
  color: #57606a;
  min-width: 120px;
}

.info-item .value {
  color: #24292f;
  flex: 1;
}

.edit-form h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0 0 24px 0;
  color: #24292f;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  font-size: 14px;
  font-weight: 600;
  color: #24292f;
  margin-bottom: 8px;
}

.form-group input,
.form-group textarea {
  width: 100%;
  padding: 12px;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  font-size: 14px;
  font-family: inherit;
  transition: border-color 0.2s;
  box-sizing: border-box;
}

.form-group input:focus,
.form-group textarea:focus {
  outline: none;
  border-color: #0969da;
  box-shadow: 0 0 0 3px rgba(9, 105, 218, 0.1);
}

.form-group textarea {
  resize: vertical;
}

.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 24px;
}

.btn-save {
  background: #2da44e;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-save:hover:not(:disabled) {
  background: #2c974b;
}

.btn-save:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-cancel {
  background: #f6f8fa;
  color: #24292f;
  border: 1px solid #d0d7de;
  padding: 10px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-cancel:hover {
  background: #f3f4f6;
  border-color: #c9d1d9;
}
</style>
