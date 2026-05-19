<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal">
      <div class="modal-header">
        <h2>Create New</h2>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>

      <div class="modal-body">
        <div class="tabs">
          <button
            class="tab"
            :class="{ active: type === 'group' }"
            @click="type = 'group'"
          >
            Group
          </button>
          <button
            class="tab"
            :class="{ active: type === 'repo' }"
            @click="type = 'repo'"
          >
            Repository
          </button>
        </div>

        <form @submit.prevent="handleSubmit">
          <div class="form-group">
            <label>Name</label>
            <input
              v-model="name"
              type="text"
              placeholder="Enter name"
              required
            />
          </div>

          <div class="form-group" v-if="type === 'group'">
            <label>Parent Path (optional)</label>
            <input
              v-model="parentPath"
              type="text"
              placeholder="e.g., team1/subgroup"
            />
          </div>

          <div class="form-group" v-if="type === 'repo'">
            <label>Group Path</label>
            <input
              v-model="groupPath"
              type="text"
              placeholder="e.g., team1"
              required
            />
          </div>

          <div class="form-group">
            <label>Description (optional)</label>
            <textarea
              v-model="description"
              placeholder="Enter description"
              rows="3"
            ></textarea>
          </div>

          <div class="form-actions">
            <button type="button" class="btn-cancel" @click="$emit('close')">
              Cancel
            </button>
            <button type="submit" class="btn-submit" :disabled="loading">
              {{ loading ? 'Creating...' : 'Create' }}
            </button>
          </div>
        </form>

        <div v-if="error" class="error-message">
          {{ error }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const emit = defineEmits(['close', 'created'])

const type = ref('group')
const name = ref('')
const parentPath = ref('')
const groupPath = ref('')
const description = ref('')
const loading = ref(false)
const error = ref('')

const handleSubmit = async () => {
  loading.value = true
  error.value = ''

  try {
    const endpoint = type.value === 'group' ? '/api/groups' : '/api/repos'
    const payload = type.value === 'group'
      ? {
          name: name.value,
          parent_path: parentPath.value || null,
          description: description.value || null
        }
      : {
          name: name.value,
          group_path: groupPath.value,
          description: description.value || null
        }

    const response = await fetch(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    })

    if (!response.ok) {
      const text = await response.text()
      throw new Error(text || 'Failed to create')
    }

    emit('created')
    emit('close')
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
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
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px;
  border-bottom: 1px solid #d0d7de;
}

.modal-header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.close-btn {
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

.close-btn:hover {
  background: #f6f8fa;
}

.modal-body {
  padding: 24px;
}

.tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 24px;
  border-bottom: 1px solid #d0d7de;
}

.tab {
  background: none;
  border: none;
  padding: 8px 16px;
  font-size: 14px;
  color: #57606a;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.tab:hover {
  color: #24292f;
}

.tab.active {
  color: #0969da;
  border-bottom-color: #0969da;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 6px;
  color: #24292f;
}

.form-group input,
.form-group textarea {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  font-size: 14px;
  font-family: inherit;
  transition: border-color 0.2s;
}

.form-group input:focus,
.form-group textarea:focus {
  outline: none;
  border-color: #0969da;
}

.form-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 24px;
}

.btn-cancel,
.btn-submit {
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
}

.btn-cancel {
  background: #f6f8fa;
  color: #24292f;
}

.btn-cancel:hover {
  background: #e1e4e8;
}

.btn-submit {
  background: #2da44e;
  color: white;
}

.btn-submit:hover:not(:disabled) {
  background: #2c974b;
}

.btn-submit:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-message {
  margin-top: 16px;
  padding: 12px;
  background: #fff1f0;
  border: 1px solid #ffccc7;
  border-radius: 6px;
  color: #cf222e;
  font-size: 14px;
}
</style>
