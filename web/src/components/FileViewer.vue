<template>
  <div class="file-viewer">
    <div class="viewer-header">
      <div class="header-left">
        <span class="file-icon">📄</span>
        <span class="file-name">{{ fileName }}</span>
        <span class="file-size" v-if="fileSize">{{ fileSize }}</span>
      </div>
      <div class="header-right">
        <button v-if="isMarkdown" class="toggle-btn" @click="showSource = !showSource">
          {{ showSource ? 'Preview' : 'Source' }}
        </button>
      </div>
    </div>
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <div v-else-if="content === null" class="empty">No content</div>
    <div v-else-if="isMarkdown && !showSource" class="markdown-rendered" v-html="renderedHtml"></div>
    <div v-else class="code-wrapper">
      <table class="code-table">
        <tbody>
          <tr v-for="(line, index) in lines" :key="index">
            <td class="line-number">{{ index + 1 }}</td>
            <td class="line-content" v-html="line"></td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import hljs from 'highlight.js'
import 'highlight.js/styles/github.css'
import { marked } from 'marked'

const props = defineProps({
  repoPath: { type: String, required: true },
  filePath: { type: String, required: true },
  branch: { type: String, default: '' }
})

const emit = defineEmits(['back'])

const content = ref(null)
const loading = ref(false)
const error = ref('')
const highlightedCode = ref('')
const showSource = ref(false)

const fileName = computed(() => {
  if (!props.filePath) return ''
  const parts = props.filePath.split('/')
  return parts[parts.length - 1]
})

const isMarkdown = computed(() => {
  return fileName.value.endsWith('.md') || fileName.value.endsWith('.markdown')
})

const renderedHtml = computed(() => {
  if (!content.value) return ''
  return marked(content.value, { breaks: true })
})

const fileSize = computed(() => {
  if (!content.value) return ''
  const bytes = content.value.length
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
})

const lines = computed(() => {
  if (!highlightedCode.value) return []
  return highlightedCode.value.split('\n')
})

const loadContent = async () => {
  loading.value = true
  error.value = ''
  showSource.value = false
  try {
    const params = new URLSearchParams({ file: props.filePath })
    if (props.branch) {
      params.append('branch', props.branch)
    }
    const url = `/api/repo-file/${props.repoPath}?${params.toString()}`
    const response = await fetch(url)
    if (!response.ok) {
      throw new Error('Failed to load file')
    }
    const data = await response.json()
    if (data.is_binary) {
      content.value = data.content
      highlightedCode.value = escapeHtml(data.content)
    } else {
      content.value = data.content
      const result = hljs.highlightAuto(data.content)
      highlightedCode.value = result.value
    }
  } catch (err) {
    error.value = err.message
    content.value = null
    highlightedCode.value = ''
  } finally {
    loading.value = false
  }
}

function escapeHtml(text) {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

watch(() => [props.repoPath, props.filePath, props.branch], () => {
  if (props.filePath) {
    loadContent()
  }
}, { immediate: true })
</script>

<style scoped>
.file-viewer {
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  overflow: hidden;
}

.viewer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid #d0d7de;
  background: #f6f8fa;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.file-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.file-name {
  font-size: 14px;
  font-weight: 600;
  color: #24292f;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  font-size: 12px;
  color: #57606a;
  flex-shrink: 0;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-btn {
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  color: #24292f;
  white-space: nowrap;
  transition: background 0.2s;
}

.toggle-btn:hover {
  background: #e1e4e8;
}

.back-btn {
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  color: #24292f;
  white-space: nowrap;
  transition: background 0.2s;
}

.back-btn:hover {
  background: #e1e4e8;
}

.markdown-rendered {
  padding: 24px 32px;
  font-size: 14px;
  line-height: 1.6;
  color: #24292f;
  overflow-x: auto;
}

.loading,
.error,
.empty {
  padding: 48px;
  text-align: center;
  color: #57606a;
  font-size: 14px;
}

.error {
  color: #cf222e;
}

.code-wrapper {
  overflow-x: auto;
  background: #fff;
}

.code-table {
  width: 100%;
  table-layout: fixed;
  border-collapse: collapse;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
}

.line-number {
  padding: 0 12px;
  text-align: right;
  color: #6e7781;
  background: #f6f8fa;
  border-right: 1px solid #d0d7de;
  user-select: none;
  vertical-align: top;
  white-space: nowrap;
  width: 50px;
}

.line-content {
  padding: 0 16px;
  white-space: pre;
  background: #fff;
  color: #24292f;
  width: auto;
  overflow-x: auto;
}

.code-table tr:hover .line-content {
  background: #f6f8fa;
}

.code-table tr:hover .line-number {
  background: #f0f2f4;
}

.markdown-rendered :deep(h1),
.markdown-rendered :deep(h2),
.markdown-rendered :deep(h3),
.markdown-rendered :deep(h4),
.markdown-rendered :deep(h5),
.markdown-rendered :deep(h6) {
  margin-top: 24px;
  margin-bottom: 16px;
  font-weight: 600;
  line-height: 1.25;
}

.markdown-rendered :deep(h1) {
  font-size: 2em;
  border-bottom: 1px solid #d0d7de;
  padding-bottom: 0.3em;
}

.markdown-rendered :deep(h2) {
  font-size: 1.5em;
  border-bottom: 1px solid #d0d7de;
  padding-bottom: 0.3em;
}

.markdown-rendered :deep(p) {
  margin-top: 0;
  margin-bottom: 16px;
}

.markdown-rendered :deep(code) {
  background: #f6f8fa;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 85%;
}

.markdown-rendered :deep(pre) {
  background: #f6f8fa;
  padding: 16px;
  border-radius: 6px;
  overflow-x: auto;
  margin-bottom: 16px;
}

.markdown-rendered :deep(pre code) {
  background: none;
  padding: 0;
}

.markdown-rendered :deep(ul),
.markdown-rendered :deep(ol) {
  padding-left: 2em;
  margin-bottom: 16px;
}

.markdown-rendered :deep(a) {
  color: #0969da;
  text-decoration: none;
}

.markdown-rendered :deep(a:hover) {
  text-decoration: underline;
}

.markdown-rendered :deep(blockquote) {
  padding: 0 1em;
  color: #57606a;
  border-left: 0.25em solid #d0d7de;
  margin-bottom: 16px;
}

.markdown-rendered :deep(img) {
  max-width: 100%;
}

.markdown-rendered :deep(table) {
  border-collapse: collapse;
  margin-bottom: 16px;
}

.markdown-rendered :deep(th),
.markdown-rendered :deep(td) {
  border: 1px solid #d0d7de;
  padding: 8px 12px;
  text-align: left;
}

.markdown-rendered :deep(th) {
  background: #f6f8fa;
  font-weight: 600;
}
</style>
