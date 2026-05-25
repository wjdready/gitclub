<template>
  <div class="tags-view">
    <div v-if="loading" class="loading">Loading tags...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <div v-else-if="tags.length === 0" class="empty-state">
      <div class="empty-icon">🏷️</div>
      <h3>No tags yet</h3>
      <p>Tags are used to mark specific points in your repository's history.</p>
    </div>
    <div v-else class="tags-container">
      <div class="tags-header">
        <h2>Tags</h2>
        <div class="tags-count">{{ tags.length }} {{ tags.length === 1 ? 'tag' : 'tags' }}</div>
      </div>
      <div class="tags-list">
        <div
          v-for="tag in tags"
          :key="tag.name"
          class="tag-item"
          @click="handleTagClick(tag)"
        >
          <div class="tag-info">
            <div class="tag-header">
              <span class="tag-icon">🏷️</span>
              <span class="tag-name">{{ tag.name }}</span>
            </div>
            <div class="tag-details">
              <span class="tag-commit" v-if="tag.commit">
                <span class="commit-icon">📝</span>
                {{ tag.commit.substring(0, 7) }}
              </span>
              <span class="tag-message" v-if="tag.message">{{ tag.message }}</span>
            </div>
          </div>
          <div class="tag-meta">
            <div class="tag-tagger" v-if="tag.tagger">
              <span class="tagger-icon">👤</span>
              {{ tag.tagger }}
            </div>
            <div class="tag-date" v-if="tag.date">{{ tag.date }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'

const props = defineProps({
  repoPath: {
    type: String,
    required: true
  }
})

const emit = defineEmits(['tag-selected'])

const tags = ref([])
const loading = ref(false)
const error = ref('')

const loadTags = async () => {
  loading.value = true
  error.value = ''

  try {
    const response = await fetch(`/api/repo/${props.repoPath}`)
    if (!response.ok) {
      throw new Error('Failed to load tags')
    }
    const data = await response.json()
    tags.value = data.tags || []
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const handleTagClick = (tag) => {
  emit('tag-selected', tag)
}

watch(() => props.repoPath, () => {
  loadTags()
}, { immediate: true })
</script>

<style scoped>
.tags-view {
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  overflow: hidden;
}

.loading,
.error {
  padding: 48px;
  text-align: center;
  color: #57606a;
}

.error {
  color: #cf222e;
}

.empty-state {
  padding: 48px;
  text-align: center;
  color: #57606a;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.empty-state h3 {
  font-size: 18px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: #24292f;
}

.empty-state p {
  font-size: 14px;
  margin: 0;
  color: #57606a;
}

.tags-container {
  padding: 24px;
}

.tags-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 1px solid #d0d7de;
}

.tags-header h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: #24292f;
}

.tags-count {
  font-size: 14px;
  color: #57606a;
  background: #f6f8fa;
  padding: 4px 12px;
  border-radius: 12px;
}

.tags-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tag-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  transition: all 0.2s;
  cursor: pointer;
}

.tag-item:hover {
  background: #f6f8fa;
  border-color: #0969da;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.tag-info {
  flex: 1;
  min-width: 0;
}

.tag-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.tag-icon {
  font-size: 18px;
  flex-shrink: 0;
}

.tag-name {
  font-size: 16px;
  font-weight: 600;
  color: #0969da;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag-details {
  display: flex;
  align-items: center;
  gap: 16px;
  font-size: 13px;
  color: #57606a;
}

.tag-commit {
  display: flex;
  align-items: center;
  gap: 4px;
  font-family: 'Courier New', monospace;
  background: #f6f8fa;
  padding: 2px 6px;
  border-radius: 3px;
}

.commit-icon {
  font-size: 12px;
}

.tag-message {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  flex-shrink: 0;
  margin-left: 16px;
}

.tag-tagger {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  color: #57606a;
}

.tagger-icon {
  font-size: 12px;
}

.tag-date {
  font-size: 12px;
  color: #57606a;
}
</style>
