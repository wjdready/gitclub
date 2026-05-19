<template>
  <div class="app" :style="{ '--sidebar-width': sidebarWidth + 'px' }">
    <header class="header">
      <div class="header-content">
        <h1 class="logo">GitClub</h1>
        <nav class="nav">
          <a href="#" class="nav-link">Repositories</a>
          <a href="#" class="nav-link">Groups</a>
        </nav>
      </div>
    </header>

    <div class="breadcrumb" v-if="selectedNode">
      <span class="breadcrumb-item" v-for="(crumb, index) in getBreadcrumbs()" :key="index">
        <span v-if="index > 0" class="breadcrumb-separator">/</span>
        <a
          href="#"
          class="breadcrumb-link"
          @click.prevent="handleBreadcrumbClick(crumb, index)"
        >
          {{ crumb.name }}
        </a>
      </span>
    </div>

    <div class="main-container">
      <aside class="sidebar">
        <div v-if="viewMode === 'tree'">
          <div class="sidebar-header">
            <h2>Groups & Repositories</h2>
            <button class="btn-new" @click="showCreateModal = true">New</button>
          </div>
          <div class="tree-container">
            <TreeNode
              v-for="node in tree"
              :key="node.path"
              :node="node"
              :selected-path="selectedPath"
              :expanded-paths="expandedPaths"
              @select="selectNode"
              @toggle="toggleNodeExpand"
            />
          </div>
        </div>

        <div v-else-if="viewMode === 'file-browser'" class="file-browser-container">
          <FileBrowser
            :repo-path="currentRepoPath"
            :current-path="currentFilePath"
            :selected-file-path="selectedFile"
            :branch="currentBranch"
            :expanded-file-paths="expandedFilePaths"
            @navigate="navigateToPath"
            @back="returnToTree"
            @toggle="toggleFilePathExpand"
            @file-select="handleFileSelected"
          />
        </div>
        <div
          class="resize-handle"
          @mousedown.prevent="startResize"
        ></div>
      </aside>

      <main class="content">
        <FileViewer
          v-if="selectedFile && selectedNode && selectedNode.is_repo"
          :repo-path="selectedNode.path"
          :file-path="selectedFile"
          :branch="currentBranch"
          @back="selectedFile = null"
        />
        <RepoDetail
          v-else-if="selectedNode && selectedNode.is_repo"
          :repo-path="selectedNode.path"
          :current-path="currentFilePath"
          :branch="currentBranch"
          @navigate="handleNavigate"
          @file-selected="handleFileSelected"
        />

        <div v-else-if="selectedNode && !selectedNode.is_repo" class="detail-panel">
          <div class="detail-header">
            <h2>{{ selectedNode.name }}</h2>
            <span class="badge group">Group</span>
          </div>

          <div class="detail-body">
            <div class="info-section">
              <h3>Information</h3>
              <div class="info-item">
                <span class="label">Path:</span>
                <code>{{ selectedNode.path }}</code>
              </div>
            </div>

            <div class="info-section">
              <h3>Group Statistics</h3>
              <div class="stats">
                <div class="stat-item">
                  <span class="stat-value">{{ countRepositories(selectedNode) }}</span>
                  <span class="stat-label">Repositories</span>
                </div>
                <div class="stat-item">
                  <span class="stat-value">{{ countSubgroups(selectedNode) }}</span>
                  <span class="stat-label">Subgroups</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="empty-state">
          <p>Select a group or repository to view details</p>
        </div>
      </main>
    </div>

    <CreateModal
      v-if="showCreateModal"
      @close="showCreateModal = false"
      @created="refreshTree"
    />
  </div>
</template>

<script setup>
import { ref, nextTick, onMounted, onUnmounted } from 'vue'
import TreeNode from './components/TreeNode.vue'
import CreateModal from './components/CreateModal.vue'
import RepoDetail from './components/RepoDetail.vue'
import FileBrowser from './components/FileBrowser.vue'
import FileViewer from './components/FileViewer.vue'

const tree = ref([])
const selectedPath = ref('')
const selectedNode = ref(null)
const showCreateModal = ref(false)
const viewMode = ref('tree')
const currentRepoPath = ref('')
const currentFilePath = ref('')
const currentBranch = ref('')
const expandedPaths = ref(new Set())
const expandedFilePaths = ref(new Set())
const selectedFile = ref(null)
const sidebarWidth = ref(320)
const MIN_SIDEBAR = 200
const MAX_SIDEBAR = 600

const scrollSelectedIntoView = () => {
  nextTick(() => {
    const selected = document.querySelector('.file-browser-container .node-content.selected')
    if (!selected) return
    const container = selected.closest('.file-tree') || selected.closest('.file-browser-container')
    if (!container) return
    const cr = container.getBoundingClientRect()
    const sr = selected.getBoundingClientRect()
    if (sr.bottom > cr.bottom || sr.top < cr.top) {
      selected.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
    }
  })
}

const startResize = (e) => {
  const startX = e.clientX
  const startWidth = sidebarWidth.value
  const onMouseMove = (ev) => {
    const newWidth = startWidth + (ev.clientX - startX)
    sidebarWidth.value = Math.max(MIN_SIDEBAR, Math.min(MAX_SIDEBAR, newWidth))
  }
  const onMouseUp = () => {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

const loadTree = async () => {
  try {
    const response = await fetch('/api/tree')
    tree.value = await response.json()
  } catch (error) {
    console.error('Failed to load tree:', error)
  }
}

const selectNode = (node) => {
  selectedPath.value = node.path
  selectedNode.value = node

  if (!node.is_repo) {
    viewMode.value = 'tree'
    currentFilePath.value = ''
  }
}

const toggleNodeExpand = (path) => {
  if (expandedPaths.value.has(path)) {
    expandedPaths.value.delete(path)
  } else {
    expandedPaths.value.add(path)
  }
}

const toggleFilePathExpand = (path) => {
  if (expandedFilePaths.value.has(path)) {
    expandedFilePaths.value.delete(path)
  } else {
    expandedFilePaths.value.add(path)
  }
}

const enterFileBrowser = (repoPath, branch) => {
  viewMode.value = 'file-browser'
  currentRepoPath.value = repoPath
  currentFilePath.value = ''
  currentBranch.value = branch || ''
  // 从组树进入文件浏览时，清除上一次的展开记录
  expandedFilePaths.value = new Set()
}

const navigateToPath = (filePath) => {
  currentFilePath.value = filePath
}

const handleNavigate = (filePath) => {
  if (viewMode.value !== 'file-browser') {
    enterFileBrowser(selectedNode.value.path, currentBranch.value)
  }
  navigateToPath(filePath)
  selectedFile.value = null
  // 只展开当前进入的目录及其必需的祖先路径
  const newExpanded = new Set()
  if (filePath) {
    const parts = filePath.split('/')
    let current = ''
    for (const part of parts) {
      current = current ? `${current}/${part}` : part
      newExpanded.add(current)
    }
  }
  expandedFilePaths.value = newExpanded
  scrollSelectedIntoView()
}

const returnToTree = () => {
  viewMode.value = 'tree'
  currentRepoPath.value = ''
  currentFilePath.value = ''
  selectedFile.value = null
}

const handleFileSelected = (file) => {
  if (viewMode.value !== 'file-browser') {
    enterFileBrowser(selectedNode.value.path, currentBranch.value)
  }
  const filePath = typeof file === 'string' ? file : file.path
  selectedFile.value = filePath
  // 更新面包屑上下文为文件所在的目录
  const lastSlash = filePath.lastIndexOf('/')
  currentFilePath.value = lastSlash >= 0 ? filePath.substring(0, lastSlash) : ''
  scrollSelectedIntoView()
}

const countRepositories = (node) => {
  let count = 0
  const traverse = (n) => {
    if (n.is_repo) {
      count++
    } else {
      n.children?.forEach(traverse)
    }
  }
  traverse(node)
  return count
}

const countSubgroups = (node) => {
  let count = 0
  const traverse = (n) => {
    if (!n.is_repo && n !== node) {
      count++
    }
    n.children?.forEach(traverse)
  }
  node.children?.forEach(traverse)
  return count
}

const refreshTree = () => {
  loadTree()
}

const getBreadcrumbs = () => {
  if (!selectedNode.value) return []

  if (viewMode.value === 'file-browser') {
    const breadcrumbs = []
    const repoName = selectedNode.value.name.replace('.git', '')
    breadcrumbs.push({
      name: repoName,
      path: '',
      isRepo: true
    })

    if (currentFilePath.value) {
      const pathParts = currentFilePath.value.split('/')
      pathParts.forEach((part, index) => {
        const fullPath = pathParts.slice(0, index + 1).join('/')
        breadcrumbs.push({
          name: part,
          path: fullPath,
          isRepo: false
        })
      })
    }

    // 如果正在查看文件，在面包屑中追加文件名
    if (selectedFile.value) {
      breadcrumbs.push({
        name: selectedFile.value.split('/').pop(),
        path: selectedFile.value,
        isRepo: false,
        isFile: true
      })
    }

    return breadcrumbs
  } else {
    return selectedNode.value.path.split('/').map(part => ({
      name: part.replace('.git', ''),
      path: '',
      isRepo: false
    }))
  }
}

const handleBreadcrumbClick = (crumb, index) => {
  if (viewMode.value === 'file-browser') {
    if (crumb.isFile) return // 点击文件名不执行任何操作
    selectedFile.value = null
    if (index === 0) {
      returnToTree()
    } else if (crumb.path !== undefined) {
      navigateToPath(crumb.path)
      scrollSelectedIntoView()
    }
  }
}

onMounted(() => {
  loadTree()
})
</script>

<style scoped>
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f6f8fa;
}

.header {
  background: #24292f;
  color: white;
  padding: 12px 32px;
  border-bottom: 1px solid #30363d;
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 100;
  height: 48px;
  box-sizing: border-box;
}

.header-content {
  display: flex;
  align-items: center;
  gap: 32px;
  height: 100%;
}

.logo {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.nav {
  display: flex;
  gap: 16px;
}

.nav-link {
  color: #c9d1d9;
  text-decoration: none;
  padding: 6px 12px;
  border-radius: 6px;
  transition: background 0.2s;
  font-size: 14px;
}

.nav-link:hover {
  background: rgba(255, 255, 255, 0.1);
  color: white;
}

.breadcrumb {
  background: #f6f8fa;
  padding: 0 32px;
  border-bottom: 1px solid #d0d7de;
  position: fixed;
  top: 48px;
  left: var(--sidebar-width, 320px);
  right: 0;
  z-index: 99;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 14px;
  height: 53px;
  box-sizing: border-box;
}

.breadcrumb-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.breadcrumb-separator {
  color: #57606a;
  margin: 0 4px;
}

.breadcrumb-link {
  color: #0969da;
  text-decoration: none;
}

.breadcrumb-link:hover {
  text-decoration: underline;
}

.main-container {
  display: flex;
  flex: 1;
  overflow: hidden;
  margin-top: 57px;
}

.main-container {
  display: flex;
  flex: 1;
  overflow: hidden;
  margin-top: 48px;
}

.sidebar {
  width: var(--sidebar-width, 320px);
  background: white;
  border-right: 1px solid #d0d7de;
  display: flex;
  flex-direction: column;
  position: fixed;
  top: 48px;
  bottom: 0;
  left: 0;
  z-index: 50;
}

.resize-handle {
  position: absolute;
  top: 0;
  right: -4px;
  width: 8px;
  height: 100%;
  cursor: col-resize;
  z-index: 60;
}

.resize-handle:hover,
.resize-handle:active {
  background: rgba(9, 105, 218, 0.15);
}

.app:has(.breadcrumb) .sidebar {
  top: 48px;
}

.app:has(.breadcrumb) .main-container {
  margin-top: 101px;
}

.app:has(.breadcrumb) .content {
  padding-top: 8px;
}

.sidebar-header {
  padding: 0 16px;
  border-bottom: 1px solid #d0d7de;
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 53px;
  box-sizing: border-box;
}

.sidebar-header h2 {
  font-size: 14px;
  font-weight: 600;
  margin: 0;
}

.btn-new {
  background: #2da44e;
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-new:hover {
  background: #2c974b;
}

.tree-container {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.file-browser-container {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.content {
  flex: 1;
  overflow-y: auto;
  margin-left: var(--sidebar-width, 320px);
  padding: 8px 24px 24px 24px;
}

.detail-panel {
  background: white;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  overflow: hidden;
}

.detail-header {
  padding: 16px 24px;
  border-bottom: 1px solid #d0d7de;
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

.detail-body {
  padding: 24px;
}

.info-section {
  margin-bottom: 24px;
}

.info-section h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
}

.info-item {
  display: flex;
  gap: 12px;
  margin-bottom: 8px;
  align-items: center;
}

.label {
  font-weight: 500;
  color: #57606a;
  min-width: 80px;
}

code {
  background: #f6f8fa;
  padding: 4px 8px;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: 13px;
}

.stats {
  display: flex;
  gap: 24px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px;
  background: #f6f8fa;
  border-radius: 6px;
  min-width: 120px;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #0969da;
}

.stat-label {
  font-size: 14px;
  color: #57606a;
  margin-top: 4px;
}

.empty-state {
  text-align: center;
  padding: 48px;
  color: #57606a;
}
</style>
