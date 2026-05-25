<template>
  <!-- 登录/注册页面 -->
  <Login
    v-if="!isAuthenticated && authView === 'login'"
    @login-success="handleLoginSuccess"
    @switch-to-register="authView = 'register'"
  />
  <Register
    v-else-if="!isAuthenticated && authView === 'register'"
    @register-success="handleRegisterSuccess"
    @switch-to-login="authView = 'login'"
  />

  <!-- 主应用界面 -->
  <div v-else class="app" :style="{ '--sidebar-width': sidebarWidth + 'px' }">
    <header class="header">
      <div class="header-content">
        <h1 class="logo">
          <a href="#" class="logo-link" @click.prevent="goHome">
            <svg class="logo-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none">
              <!-- Git 分支结构 -->
              <circle cx="12" cy="18" r="2.2" fill="white" stroke="white" stroke-width="1.5"/>
              <circle cx="7" cy="8" r="2.2" fill="white" stroke="white" stroke-width="1.5"/>
              <circle cx="17" cy="8" r="2.2" fill="white" stroke="white" stroke-width="1.5"/>
              <line x1="7" y1="10.2" x2="12" y2="15.8" stroke="white" stroke-width="2" stroke-linecap="round"/>
              <line x1="17" y1="10.2" x2="12" y2="15.8" stroke="white" stroke-width="2" stroke-linecap="round"/>
            </svg>
            <span>GitClub</span>
          </a>
        </h1>
        <nav class="nav">
          <a href="#" class="nav-link" :class="{ active: !selectedNode && currentView === 'main' }" @click.prevent="goHome">Contents</a>
        </nav>
        <div class="user-menu">
          <button class="btn-profile" @click="showProfile">
            <span class="username">{{ currentUser?.username }}</span>
          </button>
          <button class="btn-logout" @click="handleLogout">Logout</button>
        </div>
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
      <aside class="sidebar" v-if="currentView === 'main'">
        <div v-if="viewMode === 'tree'" class="tree-view-container">
          <div class="sidebar-header">
            <h2>Contents</h2>
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
        <UserProfile
          v-if="currentView === 'profile'"
          :user="currentUser"
          @profile-updated="handleProfileUpdated"
        />
        <TagsView
          v-else-if="showTagsView && selectedNode && selectedNode.is_repo"
          :repo-path="selectedNode.path"
          @tag-selected="handleTagSelected"
        />
        <FileViewer
          v-else-if="selectedFile && selectedNode && selectedNode.is_repo"
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
          @view-tags="handleViewTags"
        />

        <GroupDetail
          v-else-if="selectedNode && !selectedNode.is_repo"
          :group-path="selectedNode.path"
          :current-user="currentUser"
          @select-group="handleSelectGroup"
          @select-repo="handleSelectRepo"
          @refresh-tree="refreshTree"
        />

        <div v-else-if="!initialRouteResolved" class="empty-state">
          <p>Loading...</p>
        </div>
        <div v-else class="empty-state">
          <p>Select a group or repository to view details</p>
        </div>
      </main>
    </div>

    <CreateModal
      v-if="showCreateModal"
      :current-path="getCurrentPath()"
      @close="showCreateModal = false"
      @created="refreshTree"
    />
  </div>
</template>

<script setup>
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import TreeNode from './components/TreeNode.vue'
import CreateModal from './components/CreateModal.vue'
import RepoDetail from './components/RepoDetail.vue'
import GroupDetail from './components/GroupDetail.vue'
import FileBrowser from './components/FileBrowser.vue'
import FileViewer from './components/FileViewer.vue'
import Login from './components/Login.vue'
import Register from './components/Register.vue'
import UserProfile from './components/UserProfile.vue'
import TagsView from './components/TagsView.vue'

// 认证状态
const isAuthenticated = ref(false)
const currentUser = ref(null)
const authView = ref('login') // 'login' or 'register'
const currentView = ref('main') // 'main' or 'profile'

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
const showTagsView = ref(false)
const sidebarWidth = ref(320)
const MIN_SIDEBAR = 200
const MAX_SIDEBAR = 600
const initialRouteResolved = ref(false)
// 保存初始 URL，防止 syncUrl 的 immediate watcher 在微任务中修改 location.pathname
const initialPathname = window.location.pathname

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
  showTagsView.value = false
  // 更新面包屑上下文为文件所在的目录
  const lastSlash = filePath.lastIndexOf('/')
  currentFilePath.value = lastSlash >= 0 ? filePath.substring(0, lastSlash) : ''
  scrollSelectedIntoView()
}

const handleViewTags = () => {
  showTagsView.value = true
  selectedFile.value = null
  viewMode.value = 'tree'
}

const handleTagSelected = (tag) => {
  // 当点击某个 tag 时，可以导航到该 tag 的文件列表
  // 这里可以扩展为切换到该 tag 的分支视图
  console.log('Tag selected:', tag)
  currentBranch.value = tag.name
  showTagsView.value = false
  enterFileBrowser(selectedNode.value.path, tag.name)
}

const handleSelectRepo = (path) => {
  const targetNode = findNodeByPath(tree.value, path)
  if (targetNode) {
    selectNode(targetNode)
    const parts = path.replace(/\.git$/, '').split('/')
    for (let p = 1; p < parts.length; p++) {
      expandedPaths.value.add(parts.slice(0, p).join('/'))
    }
  }
}

const handleSelectGroup = (path) => {
  const targetNode = findNodeByPath(tree.value, path)
  if (targetNode) {
    selectNode(targetNode)
    // 展开祖先路径
    const parts = path.split('/')
    for (let p = 1; p < parts.length; p++) {
      expandedPaths.value.add(parts.slice(0, p).join('/'))
    }
  }
}

const refreshTree = () => {
  loadTree()
}

const goHome = () => {
  currentView.value = 'main'
  selectedNode.value = null
  selectedPath.value = ''
  viewMode.value = 'tree'
  currentRepoPath.value = ''
  currentFilePath.value = ''
  currentBranch.value = ''
  selectedFile.value = null
  showTagsView.value = false
  expandedPaths.value = new Set()
  expandedFilePaths.value = new Set()
}

const showProfile = () => {
  currentView.value = 'profile'
  selectedNode.value = null
  selectedPath.value = ''
  viewMode.value = 'tree'
  showTagsView.value = false
}

const handleProfileUpdated = (updatedUser) => {
  currentUser.value = updatedUser
}

const getCurrentPath = () => {
  if (!selectedNode.value) return ''
  // 如果选中的是仓库，返回其父路径（组路径）
  if (selectedNode.value.is_repo) {
    const parts = selectedNode.value.path.replace(/\.git$/, '').split('/')
    parts.pop() // 移除仓库名
    return parts.join('/')
  }
  // 如果选中的是组，返回组路径
  return selectedNode.value.path
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
    const parts = selectedNode.value.path.split('/')
    return parts.map((part, index) => ({
      name: part.replace('.git', ''),
      path: parts.slice(0, index + 1).join('/'),
      isRepo: index === parts.length - 1 ? selectedNode.value.is_repo : false
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
  } else if (crumb.path) {
    // 树视图模式：点击面包屑段导航到对应组/仓库
    const targetNode = findNodeByPath(tree.value, crumb.path)
    if (targetNode) {
      selectNode(targetNode)
      // 展开祖先路径
      const parts = crumb.path.split('/')
      for (let p = 1; p < parts.length; p++) {
        expandedPaths.value.add(parts.slice(0, p).join('/'))
      }
    }
  }
}

function syncUrl() {
  let path = ''
  const cleanRepoPath = selectedNode.value?.path?.replace(/\.git$/, '') || ''
  if (showTagsView.value && selectedNode.value?.is_repo) {
    path = `/${cleanRepoPath}/tags`
  } else if (viewMode.value === 'file-browser' && selectedNode.value?.is_repo) {
    if (selectedFile.value) {
      path = `/${cleanRepoPath}/blob/${currentBranch.value || 'HEAD'}/${selectedFile.value}`
    } else if (currentFilePath.value) {
      path = `/${cleanRepoPath}/tree/${currentBranch.value || 'HEAD'}/${currentFilePath.value}`
    } else {
      path = `/${cleanRepoPath}/tree`
    }
  } else if (selectedNode.value) {
    path = '/' + cleanRepoPath
  }
  // 对路径中的每个部分进行编码，但保留斜杠
  const url = path ? path.split('/').map(segment => encodeURIComponent(segment)).join('/') : '/'
  window.history.replaceState(null, '', url)
}

function findNodeByPath(nodes, targetPath) {
  for (const node of nodes) {
    if (node.path === targetPath) return node
    if (node.children) {
      const found = findNodeByPath(node.children, targetPath)
      if (found) return found
    }
  }
  return null
}

function resolveRoute() {
  const pathname = initialPathname.replace(/\/$/, '') || ''
  if (!pathname) return
  console.log('Resolving route:', pathname)

  // 解码 URL 中的中文字符
  const decodedPathname = decodeURIComponent(pathname)
  const segments = decodedPathname.split('/').filter(Boolean)
  console.log('Segments:', segments)

  // 尝试从长到短匹配仓库路径（仓库路径可能包含斜杠）
  for (let i = segments.length; i >= 1; i--) {
    const candidatePath = segments.slice(0, i).join('/')
    // 先尝试加上 .git 匹配（标准 URL 格式，如 /backend/some/myproject）
    let repoPath = candidatePath + '.git'
    let repo = findNodeByPath(tree.value, repoPath)
    // 如果没找到，并且 URL 中已经包含 .git，则直接尝试匹配
    if (!repo && candidatePath.endsWith('.git')) {
      repoPath = candidatePath
      repo = findNodeByPath(tree.value, repoPath)
    }
    console.log('Trying repo:', repoPath, repo ? 'found' : 'not found')
    if (repo) {
      console.log('Found repo:', repo.path)
      selectedPath.value = repo.path
      selectedNode.value = repo

      // 展开左侧组树到当前节点
      const candidateParts = candidatePath.split('/')
      for (let p = 1; p < candidateParts.length; p++) {
        expandedPaths.value.add(candidateParts.slice(0, p).join('/'))
      }

      const remaining = segments.slice(i)

      if (remaining.length >= 1) {
        const action = remaining[0] // blob, tree, 或 tags

        if (action === 'tags') {
          // 显示 tags 视图
          showTagsView.value = true
          selectedFile.value = null
          viewMode.value = 'tree'
          return
        }

        // 有子路径（/tree/... 或 /blob/...），进入文件浏览模式
        enterFileBrowser(repo.path, '')
        const branch = remaining[1] || ''
        const filePath = remaining.slice(2).join('/')
        currentBranch.value = branch === 'HEAD' ? '' : branch
        if (filePath) {
          if (action === 'blob') {
            selectedFile.value = filePath
            const lastSlash = filePath.lastIndexOf('/')
            currentFilePath.value = lastSlash >= 0 ? filePath.substring(0, lastSlash) : ''
            // 展开文件所在目录
            const newExpanded = new Set()
            if (currentFilePath.value) {
              const parts = currentFilePath.value.split('/')
              let current = ''
              for (const part of parts) {
                current = current ? `${current}/${part}` : part
                newExpanded.add(current)
              }
            }
            expandedFilePaths.value = newExpanded
          } else {
            handleNavigate(filePath)
          }
        }
      }
      // 没有子路径：URL 只是仓库路径，保持组树视图
      return
    }
  }

  // 没找到仓库，尝试匹配组节点
  for (let i = segments.length; i >= 1; i--) {
    const groupPath = segments.slice(0, i).join('/')
    console.log('Trying group:', groupPath)
    const group = findNodeByPath(tree.value, groupPath)
    if (group) {
      console.log('Found group:', group.path)
      selectNode(group)
      // 展开左侧组树到当前节点
      const groupParts = groupPath.split('/')
      for (let p = 1; p < groupParts.length; p++) {
        expandedPaths.value.add(groupParts.slice(0, p).join('/'))
      }
      return
    }
  }
  console.log('No matching route found for:', pathname)
}

// 认证相关函数
const checkAuth = async () => {
  try {
    // 尝试获取当前用户信息（通过 cookie 验证）
    const response = await fetch('/api/auth/me')
    if (response.ok) {
      const data = await response.json()
      if (data.success && data.user) {
        isAuthenticated.value = true
        currentUser.value = data.user
        return true
      }
    }
  } catch (err) {
    console.error('Auth check failed:', err)
  }
  isAuthenticated.value = false
  currentUser.value = null
  return false
}

const handleLoginSuccess = (user) => {
  isAuthenticated.value = true
  currentUser.value = user
  loadTree()
  resolveRoute()
}

const handleRegisterSuccess = (user) => {
  // 注册成功后切换到登录页面
  authView.value = 'login'
}

const handleLogout = async () => {
  try {
    await fetch('/api/auth/logout', { method: 'POST' })
  } catch (err) {
    console.error('Logout failed:', err)
  }
  isAuthenticated.value = false
  currentUser.value = null
  goHome()
}

watch(
  [viewMode, selectedFile, currentFilePath, currentBranch, selectedNode, showTagsView],
  () => nextTick(syncUrl),
  { immediate: true }
)

// 当树数据加载后，重新尝试解析路由（处理边缘情况）
watch(tree, (newTree) => {
  if (!initialRouteResolved.value && newTree.length > 0 && initialPathname !== '/') {
    console.log('Tree loaded, re-resolving route...')
    resolveRoute()
    initialRouteResolved.value = true
  }
})

onMounted(async () => {
  // 先检查认证状态
  const authenticated = await checkAuth()
  if (authenticated) {
    await loadTree()
    resolveRoute()
    initialRouteResolved.value = true
  }
})
</script>

<style scoped>
.app {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f6f8fa;
  overflow: hidden;
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

.logo-link {
  display: flex;
  align-items: center;
  gap: 10px;
  color: white;
  text-decoration: none;
  transition: opacity 0.2s;
}

.logo-link:hover {
  opacity: 0.8;
}

.logo-icon {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
}

.nav {
  display: flex;
  gap: 16px;
  flex: 1;
}

.user-menu {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-left: auto;
}

.btn-profile {
  background: transparent;
  color: #c9d1d9;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-profile:hover {
  background: rgba(255, 255, 255, 0.1);
  color: white;
}

.username {
  color: #c9d1d9;
  font-size: 14px;
}

.btn-logout {
  background: rgba(255, 255, 255, 0.1);
  color: #c9d1d9;
  border: 1px solid rgba(255, 255, 255, 0.2);
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-logout:hover {
  background: rgba(255, 255, 255, 0.15);
  color: white;
  border-color: rgba(255, 255, 255, 0.3);
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

.nav-link.active {
  background: rgba(255, 255, 255, 0.15);
  color: white;
  font-weight: 500;
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
  margin-top: 48px;
  min-height: 0;
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
  overflow: hidden;
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
  top: 101px;
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
  min-height: 0;
}

.tree-view-container {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.file-browser-container {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.content {
  position: fixed;
  top: 48px;
  bottom: 0;
  left: var(--sidebar-width, 320px);
  right: 0;
  overflow-y: auto;
  padding: 8px 24px 24px 24px;
}

.app:has(.sidebar:not(:empty)) .content {
  left: var(--sidebar-width, 320px);
}

.app:not(:has(.sidebar)) .content,
.app:has(.sidebar:empty) .content {
  left: 0;
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
