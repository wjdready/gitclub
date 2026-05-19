<template>
  <div class="app">
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
      <span class="breadcrumb-item" v-for="(part, index) in getBreadcrumbs()" :key="index">
        <span v-if="index > 0" class="breadcrumb-separator">/</span>
        <a href="#" class="breadcrumb-link">{{ part }}</a>
      </span>
    </div>

    <div class="main-container">
      <aside class="sidebar">
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
            @select="selectNode"
          />
        </div>
      </aside>

      <main class="content">
        <RepoDetail v-if="selectedNode && selectedNode.is_repo" :repo-path="selectedNode.path" />

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
import { ref, onMounted } from 'vue'
import TreeNode from './components/TreeNode.vue'
import CreateModal from './components/CreateModal.vue'
import RepoDetail from './components/RepoDetail.vue'

const tree = ref([])
const selectedPath = ref('')
const selectedNode = ref(null)
const showCreateModal = ref(false)

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
  return selectedNode.value.path.split('/').map(part => {
    if (part.endsWith('.git')) {
      return part.slice(0, -4)
    }
    return part
  })
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
  left: 320px;
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
  width: 320px;
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

.content {
  flex: 1;
  overflow-y: auto;
  margin-left: 320px;
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
