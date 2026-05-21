<template>
  <div class="repo-detail">
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <div v-else-if="detail">
      <div class="repo-content">
        <div class="main-content">
          <div class="repo-header">
            <div class="header-left">
              <h2>{{ detail.name }}</h2>
            </div>
            <div class="header-right">
              <button class="action-button star-button">
                <span class="button-icon">⭐</span>
                <span>Star</span>
              </button>
              <button class="action-button fork-button">
                <span class="button-icon">🔱</span>
                <span>Fork</span>
              </button>
              <div class="branch-selector" @click="toggleBranchDropdown">
                <span class="branch-icon">⎇</span>
                <span class="branch-name">{{ detail.default_branch || 'main' }}</span>
                <span class="dropdown-arrow">▼</span>
                <div v-if="showBranchDropdown" class="branch-dropdown" @click.stop>
                  <div class="dropdown-header">Switch branches</div>
                  <div class="dropdown-list">
                    <div
                      v-for="branch in detail.branches"
                      :key="branch"
                      class="dropdown-item"
                      :class="{ active: branch === detail.default_branch }"
                      @click="selectBranch(branch)"
                    >
                      <span class="check-icon">{{ branch === detail.default_branch ? '✓' : '' }}</span>
                      {{ branch }}
                    </div>
                  </div>
                </div>
              </div>
              <div class="code-button" @click="toggleCodeDropdown">
                <span class="code-icon">⬇</span>
                <span>Code</span>
                <span class="dropdown-arrow">▼</span>
                <div v-if="showCodeDropdown" class="code-dropdown" @click.stop>
                  <div class="dropdown-section">
                    <div class="section-title">Clone</div>
                    <div class="clone-option">
                      <div class="clone-label">HTTPS</div>
                      <div class="clone-url">
                        <input
                          ref="httpsInput"
                          type="text"
                          :value="getCloneUrl(detail.path)"
                          readonly
                        />
                        <button class="copy-btn" @click="copyToClipboard(getCloneUrl(detail.path))">
                          📋
                        </button>
                      </div>
                    </div>
                    <div class="clone-option">
                      <div class="clone-label">SSH</div>
                      <div class="clone-url">
                        <input
                          ref="sshInput"
                          type="text"
                          :value="getSshUrl(detail.path)"
                          readonly
                        />
                        <button class="copy-btn" @click="copyToClipboard(getSshUrl(detail.path))">
                          📋
                        </button>
                      </div>
                    </div>
                  </div>
                  <div class="dropdown-divider"></div>
                  <div class="dropdown-section">
                    <a :href="getDownloadUrl(detail.path)" class="download-link">
                      <span class="download-icon">📦</span>
                      Download ZIP
                    </a>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="files-section">
            <div v-if="detail.files.length === 0" class="empty-repo">
              <div class="empty-icon">📦</div>
              <h3>create a new repository on the command line</h3>
              <div class="code-block">
                <pre><code>echo "# {{ detail.name }}" >> README.md
git init
git add README.md
git commit -m "first commit"
git branch -M main
git remote add origin {{ getCloneUrl(detail.path) }}
git push -u origin main</code></pre>
                <button class="copy-code-btn" @click="copyCreateRepoCommands">
                  📋
                </button>
              </div>

              <h3>or push an existing repository from the command line</h3>
              <div class="code-block">
                <pre><code>git remote add origin {{ getCloneUrl(detail.path) }}
git branch -M main
git push -u origin main</code></pre>
                <button class="copy-code-btn" @click="copyPushCommands">
                  📋
                </button>
              </div>
            </div>
            <div v-else>
              <div class="files-header">
                <div class="header-col col-name">名称</div>
                <div class="header-col col-message">最后提交</div>
                <div class="header-col col-date">最后更新</div>
                <div class="header-col col-size">大小</div>
              </div>
              <div class="files-list">
                <div
                  v-for="file in detail.files"
                  :key="file.path"
                  class="file-item"
                  @click="handleFileClick(file)"
                >
                  <div class="file-info">
                    <span class="file-icon">{{ file.is_dir ? '📁' : '📄' }}</span>
                    <span class="file-name">{{ file.name }}</span>
                  </div>
                  <div class="file-message">{{ file.commit_message || '' }}</div>
                  <div class="file-date">{{ file.commit_date || '' }}</div>
                  <div class="file-size">{{ file.is_dir ? '-' : formatSize(file.size) }}</div>
                </div>
              </div>
            </div>
          </div>

          <div class="readme-section" v-if="detail.readme_content">
            <div class="markdown-body" v-html="renderedReadme"></div>
          </div>
        </div>

        <aside class="sidebar-info">
          <div class="info-section">
            <h3>About</h3>
            <p class="description">{{ detail.description || 'No description, website, or topics provided.' }}</p>

            <div class="info-links">
              <a href="#" class="info-link">
                <span class="icon">📖</span>
                Readme
              </a>
              <a href="#" class="info-link" v-if="detail.license">
                <span class="icon">⚖️</span>
                {{ detail.license }} license
              </a>
              <a href="#" class="info-link">
                <span class="icon">📊</span>
                Activity
              </a>
            </div>

            <div class="stats">
              <div class="stat-item">
                <span class="icon">⭐</span>
                <span>0 stars</span>
              </div>
              <div class="stat-item">
                <span class="icon">👁️</span>
                <span>0 watching</span>
              </div>
              <div class="stat-item">
                <span class="icon">🔱</span>
                <span>0 forks</span>
              </div>
            </div>
          </div>

          <div class="info-section">
            <h3>Releases</h3>
            <p class="empty-text">No releases published</p>
            <a href="#" class="create-link">Create a new release</a>
          </div>

          <div class="info-section">
            <h3>Packages</h3>
            <p class="empty-text">No packages published</p>
          </div>

          <div class="info-section" v-if="detail.languages && detail.languages.length > 0">
            <h3>Languages</h3>
            <div class="languages">
              <div v-for="lang in detail.languages" :key="lang.name" class="language-item">
                <span class="language-dot" :style="{ background: lang.color }"></span>
                <span class="language-name">{{ lang.name }}</span>
                <span class="language-percent">{{ lang.percent }}%</span>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import { marked } from 'marked'

const props = defineProps({
  repoPath: {
    type: String,
    required: true
  },
  currentPath: {
    type: String,
    default: ''
  },
  branch: {
    type: String,
    default: ''
  }
})

const emit = defineEmits(['navigate', 'file-selected'])

const detail = ref(null)
const loading = ref(false)
const error = ref('')
const showBranchDropdown = ref(false)
const showCodeDropdown = ref(false)
const httpsInput = ref(null)
const sshInput = ref(null)

const renderedReadme = computed(() => {
  if (!detail.value?.readme_content) return ''
  return marked(detail.value.readme_content)
})

const loadDetail = async () => {
  loading.value = true
  error.value = ''

  try {
    const params = new URLSearchParams()
    if (props.currentPath) {
      params.append('path', props.currentPath)
    }
    if (props.branch) {
      params.append('branch', props.branch)
    }

    const url = `/api/repo/${props.repoPath}${params.toString() ? '?' + params.toString() : ''}`
    console.log('Loading repo detail from URL:', url)
    console.log('props.repoPath:', props.repoPath)
    const response = await fetch(url)
    if (!response.ok) {
      throw new Error('Failed to load repository details')
    }
    detail.value = await response.json()
    console.log('Loaded detail:', detail.value)
    console.log('Files count:', detail.value.files.length)
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const getCloneUrl = (path) => {
  return `${window.location.origin}/${path}`
}

const getSshUrl = (path) => {
  const host = window.location.hostname
  return `git@${host}:${path}`
}

const getDownloadUrl = (path) => {
  return `/api/repo/${path}/archive/zip`
}

const toggleBranchDropdown = () => {
  showBranchDropdown.value = !showBranchDropdown.value
  showCodeDropdown.value = false
}

const toggleCodeDropdown = () => {
  showCodeDropdown.value = !showCodeDropdown.value
  showBranchDropdown.value = false
}

const selectBranch = (branch) => {
  // TODO: 切换分支并重新加载文件列表
  showBranchDropdown.value = false
}

const copyToClipboard = async (text) => {
  try {
    await navigator.clipboard.writeText(text)
  } catch (err) {
    console.error('Failed to copy:', err)
  }
}

const copyCreateRepoCommands = () => {
  const commands = `echo "# ${detail.value.name}" >> README.md
git init
git add README.md
git commit -m "first commit"
git branch -M main
git remote add origin ${getCloneUrl(detail.value.path)}
git push -u origin main`
  copyToClipboard(commands)
}

const copyPushCommands = () => {
  const commands = `git remote add origin ${getCloneUrl(detail.value.path)}
git branch -M main
git push -u origin main`
  copyToClipboard(commands)
}

const handleFileClick = (file) => {
  if (file.is_dir) {
    emit('navigate', file.path)
  } else {
    emit('file-selected', file)
  }
}

const formatSize = (bytes) => {
  if (bytes === 0) return '-'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

watch(() => props.repoPath, () => {
  loadDetail()
}, { immediate: true })

watch(() => [props.currentPath, props.branch], () => {
  loadDetail()
})
</script>

<style scoped>
.repo-detail {
  background: transparent;
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

.repo-header {
  padding: 4px 0 0 0;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.repo-header h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.header-right {
  display: flex;
  gap: 8px;
}

.action-button {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s;
  color: #24292f;
}

.action-button:hover {
  background: #e1e4e8;
}

.button-icon {
  font-size: 14px;
}

.branch-selector,
.code-button {
  position: relative;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.branch-selector:hover,
.code-button:hover {
  background: #e1e4e8;
}

.code-button {
  background: #2da44e;
  color: white;
  border-color: #2da44e;
}

.code-button:hover {
  background: #2c974b;
}

.branch-icon,
.code-icon {
  font-size: 14px;
}

.dropdown-arrow {
  font-size: 10px;
  color: #57606a;
}

.code-button .dropdown-arrow {
  color: white;
}

.branch-dropdown,
.code-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  min-width: 300px;
  z-index: 100;
}

.dropdown-header {
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 600;
  color: #57606a;
  border-bottom: 1px solid #d0d7de;
}

.dropdown-list {
  max-height: 300px;
  overflow-y: auto;
}

.dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.dropdown-item:hover {
  background: #f6f8fa;
}

.dropdown-item.active {
  background: #ddf4ff;
  color: #0969da;
  font-weight: 500;
}

.check-icon {
  width: 16px;
  font-size: 12px;
}

.dropdown-section {
  padding: 12px 16px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: #57606a;
  margin-bottom: 8px;
}

.clone-option {
  margin-bottom: 12px;
}

.clone-option:last-child {
  margin-bottom: 0;
}

.clone-label {
  font-size: 12px;
  font-weight: 600;
  color: #24292f;
  margin-bottom: 4px;
}

.clone-url {
  display: flex;
  gap: 4px;
}

.clone-url input {
  flex: 1;
  padding: 6px 8px;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  font-size: 12px;
  font-family: 'Courier New', monospace;
  background: #f6f8fa;
}

.copy-btn {
  padding: 6px 12px;
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s;
}

.copy-btn:hover {
  background: #e1e4e8;
}

.dropdown-divider {
  height: 1px;
  background: #d0d7de;
  margin: 8px 0;
}

.download-link {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  color: #24292f;
  text-decoration: none;
  border-radius: 6px;
  transition: background 0.2s;
}

.download-link:hover {
  background: #f6f8fa;
}

.download-icon {
  font-size: 16px;
}

.repo-content {
  display: flex;
  gap: 24px;
  padding: 8px 24px 24px 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.main-content {
  flex: 1;
  min-width: 0;
}

.sidebar-info {
  width: 250px;
  flex-shrink: 0;
  margin-top: 40px;
}

.files-section,
.readme-section {
  padding: 0;
}

.files-section {
  border: 1px solid #d0d7de;
  border-radius: 6px;
  overflow: hidden;
  margin-bottom: 8px;
  background: white;
}

.files-header {
  display: grid;
  grid-template-columns: 2fr 2fr 1fr 100px;
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

.empty-files {
  padding: 48px;
  text-align: center;
  color: #57606a;
}

.empty-repo {
  padding: 32px 48px;
  color: #24292f;
}

.empty-icon {
  font-size: 48px;
  text-align: center;
  margin-bottom: 16px;
}

.empty-repo h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 24px 0 12px 0;
  color: #24292f;
}

.empty-repo h3:first-of-type {
  margin-top: 0;
}

.setup-divider {
  text-align: center;
  margin: 24px 0;
  position: relative;
}

.setup-divider::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 1px;
  background: #d0d7de;
}

.setup-divider span {
  position: relative;
  background: white;
  padding: 0 16px;
  color: #57606a;
  font-size: 14px;
  font-weight: 600;
}

.code-block {
  position: relative;
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  padding: 16px;
  margin-bottom: 16px;
  max-width: 800px;
}

.code-block pre {
  margin: 0;
  overflow-x: auto;
}

.code-block code {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  color: #24292f;
  white-space: pre;
}

.copy-code-btn {
  position: absolute;
  top: 12px;
  right: 12px;
  padding: 6px 12px;
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s;
  font-size: 14px;
}

.copy-code-btn:hover {
  background: #f6f8fa;
}

.files-list {
  display: flex;
  flex-direction: column;
}

.file-item {
  display: grid;
  grid-template-columns: 2fr 2fr 1fr 100px;
  gap: 16px;
  align-items: center;
  padding: 8px 16px;
  border-bottom: 1px solid #d0d7de;
  transition: background 0.2s;
  cursor: pointer;
}

.file-item:last-child {
  border-bottom: none;
}

.file-item:hover {
  background: #f6f8fa;
}

.file-info {
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
  color: #0969da;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-message {
  font-size: 13px;
  color: #57606a;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-date {
  font-size: 13px;
  color: #57606a;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  font-size: 13px;
  color: #57606a;
  text-align: right;
}

.readme-section {
  border: 1px solid #d0d7de;
  border-radius: 6px;
  padding: 24px;
  background: white;
}

.info-section {
  margin-bottom: 24px;
  padding-bottom: 24px;
  border-bottom: 1px solid #d0d7de;
}

.info-section:last-child {
  border-bottom: none;
  margin-bottom: 0;
  padding-bottom: 0;
}

.info-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: #24292f;
}

.description {
  font-size: 14px;
  color: #57606a;
  margin: 0 0 16px 0;
  line-height: 1.5;
}

.info-links {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.info-link {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #0969da;
  text-decoration: none;
  padding: 4px 0;
}

.info-link:hover {
  text-decoration: underline;
}

.info-link .icon {
  font-size: 16px;
}

.stats {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #57606a;
}

.stat-item .icon {
  font-size: 14px;
}

.empty-text {
  font-size: 12px;
  color: #57606a;
  margin: 0 0 8px 0;
}

.create-link {
  font-size: 12px;
  color: #0969da;
  text-decoration: none;
}

.create-link:hover {
  text-decoration: underline;
}

.languages {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.language-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.language-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.language-name {
  flex: 1;
  color: #24292f;
}

.language-percent {
  color: #57606a;
}

.markdown-body {
  font-size: 14px;
  line-height: 1.6;
  color: #24292f;
}

.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4),
.markdown-body :deep(h5),
.markdown-body :deep(h6) {
  margin-top: 24px;
  margin-bottom: 16px;
  font-weight: 600;
  line-height: 1.25;
}

.markdown-body :deep(h1) {
  font-size: 2em;
  border-bottom: 1px solid #d0d7de;
  padding-bottom: 0.3em;
}

.markdown-body :deep(h2) {
  font-size: 1.5em;
  border-bottom: 1px solid #d0d7de;
  padding-bottom: 0.3em;
}

.markdown-body :deep(p) {
  margin-top: 0;
  margin-bottom: 16px;
}

.markdown-body :deep(code) {
  background: #f6f8fa;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Courier New', monospace;
  font-size: 85%;
}

.markdown-body :deep(pre) {
  background: #f6f8fa;
  padding: 16px;
  border-radius: 6px;
  overflow-x: auto;
  margin-bottom: 16px;
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 2em;
  margin-bottom: 16px;
}

.markdown-body :deep(a) {
  color: #0969da;
  text-decoration: none;
}

.markdown-body :deep(a:hover) {
  text-decoration: underline;
}

.markdown-body :deep(blockquote) {
  padding: 0 1em;
  color: #57606a;
  border-left: 0.25em solid #d0d7de;
  margin-bottom: 16px;
}
</style>
