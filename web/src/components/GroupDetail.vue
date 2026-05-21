<template>
  <div class="group-detail">
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <div v-else-if="detail">
      <div class="group-content">
        <div class="content-layout">
          <div class="main-content">
            <div class="detail-header">
              <h2>{{ detail.name }}</h2>
              <span class="badge group">Group</span>
            </div>

            <div class="info-section">
              <h3>Information</h3>
              <div class="info-grid">
                <div class="info-item">
                  <span class="label">Path:</span>
                  <code>{{ detail.path }}</code>
                </div>
                <div class="info-item">
                  <span class="label">Disk usage:</span>
                  <span class="size-value">{{ detail.total_size_str }}</span>
                </div>
                <div v-if="detail.description" class="info-item">
                  <span class="label">Description:</span>
                  <span class="description-value">{{ detail.description }}</span>
                </div>
              </div>
            </div>

            <div v-if="detail.repositories.length > 0 || detail.subgroups.length > 0" class="items-section">
              <h3>Contents ({{ detail.subgroups.length + detail.repositories.length }})</h3>
              <div class="items-table">
                <div class="items-header">
                  <div class="header-col col-name">Name</div>
                  <div class="header-col col-branch">Default branch</div>
                  <div class="header-col col-message">Last commit</div>
                  <div class="header-col col-date">Time</div>
                  <div class="header-col col-size">Size</div>
                </div>
                <div class="items-list">
                  <div
                    v-for="sub in detail.subgroups"
                    :key="sub.path"
                    class="item"
                    @click="selectSubgroup(sub)"
                  >
                    <div class="item-info">
                      <span class="item-icon">📁</span>
                      <span class="item-name">{{ sub.name }}</span>
                    </div>
                    <div class="item-branch"></div>
                    <div class="item-message"></div>
                    <div class="item-date"></div>
                    <div class="item-size">{{ sub.size_str }}</div>
                  </div>
                  <div
                    v-for="repo in detail.repositories"
                    :key="repo.path"
                    class="item"
                    @click="selectRepo(repo)"
                  >
                    <div class="item-info">
                      <span class="item-icon">📦</span>
                      <span class="item-name">{{ repo.name }}</span>
                    </div>
                    <div class="item-branch">{{ repo.default_branch || '' }}</div>
                    <div class="item-message">{{ repo.last_commit_message || '' }}</div>
                    <div class="item-date">{{ repo.last_commit_date || '' }}</div>
                    <div class="item-size">{{ repo.size_str }}</div>
                  </div>
                </div>
              </div>
            </div>

            <div v-else class="empty">
              This group is empty
            </div>
          </div>

          <aside class="sidebar-right">
            <MemberManagement
              :owner="detail.owner"
              :members="detail.members"
              :resource-path="detail.path"
              resource-type="group"
              :can-manage="canManageMembers"
              @member-added="handleMemberChanged"
              @member-removed="handleMemberChanged"
            />
          </aside>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed } from 'vue'
import MemberManagement from './MemberManagement.vue'

const props = defineProps({
  groupPath: {
    type: String,
    required: true
  },
  currentUser: Object,
})

const emit = defineEmits(['select-group', 'select-repo', 'refresh-tree'])

const detail = ref(null)
const loading = ref(false)
const error = ref('')

const canManageMembers = computed(() => {
  if (!props.currentUser) return false
  // 管理员总是可以管理
  if (props.currentUser.is_admin) return true
  // 如果有所有者，检查是否是所有者
  if (detail.value && detail.value.owner && detail.value.owner.user_id === props.currentUser.id) return true
  return false
})

const loadDetail = async () => {
  loading.value = true
  error.value = ''

  try {
    const response = await fetch(`/api/group/${props.groupPath}`)
    if (!response.ok) {
      throw new Error('Failed to load group details')
    }
    detail.value = await response.json()
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const handleMemberChanged = () => {
  loadDetail()
  emit('refresh-tree')
}

const selectSubgroup = (sub) => {
  emit('select-group', sub.path)
}

const selectRepo = (repo) => {
  emit('select-repo', repo.path)
}

watch(() => props.groupPath, () => {
  loadDetail()
}, { immediate: true })
</script>

<style scoped>
.group-detail {
  background: transparent;
  min-height: 100%;
}

.loading,
.error,
.empty {
  padding: 48px;
  text-align: center;
  color: #57606a;
}

.error {
  color: #cf222e;
}

.group-content {
  padding: 8px 24px 24px 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.content-layout {
  display: flex;
  gap: 24px;
}

.main-content {
  flex: 1;
  min-width: 0;
}

.sidebar-right {
  width: 320px;
  flex-shrink: 0;
}

.detail-header {
  padding: 4px 0 0 0;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
}

.detail-header h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.badge {
  background: #0969da;
  color: white;
  padding: 4px 8px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.badge.group {
  background: #8250df;
}

.info-section {
  margin-bottom: 24px;
}

.info-section h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
}

.info-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.info-item {
  display: flex;
  gap: 12px;
  align-items: center;
}

.label {
  font-weight: 500;
  color: #57606a;
  min-width: 100px;
}

code {
  background: #f6f8fa;
  padding: 4px 8px;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: 13px;
}

.size-value {
  font-size: 14px;
  font-weight: 500;
  color: #24292f;
}

.description-value {
  font-size: 14px;
  color: #24292f;
}

.items-section {
  margin-bottom: 24px;
}

.items-section h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
}

.items-table {
  border: 1px solid #d0d7de;
  border-radius: 6px;
  overflow: hidden;
  background: white;
}

.items-header {
  display: grid;
  grid-template-columns: 2fr 1fr 2fr 1fr 150px;
  gap: 16px;
  padding: 8px 16px;
  background: #f6f8fa;
  border-bottom: 1px solid #d0d7de;
  font-size: 12px;
  font-weight: 600;
  color: #57606a;
}

.header-col {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.col-size {
  text-align: right;
}

.items-list {
  display: flex;
  flex-direction: column;
}

.item {
  display: grid;
  grid-template-columns: 2fr 1fr 2fr 1fr 150px;
  gap: 16px;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid #d0d7de;
  cursor: pointer;
  transition: background 0.2s;
}

.item:last-child {
  border-bottom: none;
}

.item:hover {
  background: #f6f8fa;
}

.item-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.item-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.item-name {
  font-size: 14px;
  color: #0969da;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-size {
  font-size: 13px;
  color: #57606a;
  text-align: right;
}

.item-branch {
  font-size: 13px;
  color: #57606a;
}

.item-message {
  font-size: 13px;
  color: #57606a;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-date {
  font-size: 13px;
  color: #57606a;
}
</style>
