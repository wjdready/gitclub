<template>
  <div class="file-browser">
    <div class="browser-header">
      <button @click="$emit('back')" class="back-button">
        ← Back to Tree
      </button>
    </div>
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else class="file-tree">
      <FileBrowserNode
        v-for="item in fileTree"
        :key="item.path"
        :item="item"
        :current-path="currentPath"
        :selected-file-path="selectedFilePath"
        :repo-path="repoPath"
        :branch="branch"
        :expanded-file-paths="expandedFilePaths"
        @navigate="$emit('navigate', $event)"
        @toggle="$emit('toggle', $event)"
        @file-select="$emit('file-select', $event)"
      />
    </div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import FileBrowserNode from './FileBrowserNode.vue'

const props = defineProps({
  repoPath: {
    type: String,
    required: true
  },
  currentPath: {
    type: String,
    default: ''
  },
  selectedFilePath: {
    type: String,
    default: ''
  },
  branch: {
    type: String,
    default: 'main'
  },
  expandedFilePaths: {
    type: Set,
    required: true
  }
})

const emit = defineEmits(['navigate', 'back', 'toggle', 'file-select'])

const fileTree = ref([])
const loading = ref(false)

const loadFileTree = async () => {
  loading.value = true
  try {
    console.log('FileBrowser loading:', props.repoPath, props.branch)
    const url = `/api/repo/${props.repoPath}${props.branch ? '?branch=' + props.branch : ''}`
    console.log('FileBrowser URL:', url)
    const response = await fetch(url)
    const data = await response.json()
    console.log('FileBrowser data:', data)
    console.log('FileBrowser files:', data.files)
    // 直接使用 API 返回的文件列表，不需要构建树
    fileTree.value = data.files.map(file => ({
      name: file.name,
      path: file.path,
      is_dir: file.is_dir,
      children: []
    }))
    console.log('FileBrowser tree:', fileTree.value)
  } catch (error) {
    console.error('Failed to load file tree:', error)
  } finally {
    loading.value = false
  }
}

const buildTree = (files) => {
  const root = []

  files.forEach(file => {
    const parts = file.path.split('/')
    let current = root

    parts.forEach((part, index) => {
      if (index === parts.length - 1) {
        current.push({
          name: part,
          path: file.path,
          is_dir: file.is_dir,
          children: []
        })
      } else {
        let dir = current.find(item => item.name === part && item.is_dir)
        if (!dir) {
          const dirPath = parts.slice(0, index + 1).join('/')
          dir = {
            name: part,
            path: dirPath,
            is_dir: true,
            children: []
          }
          current.push(dir)
        }
        current = dir.children
      }
    })
  })

  return root
}

watch(() => [props.repoPath, props.branch], () => {
  loadFileTree()
}, { immediate: true })
</script>

<style scoped>
.file-browser {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.browser-header {
  padding: 0 16px;
  border-bottom: 1px solid #d0d7de;
  display: flex;
  align-items: center;
  height: 53px;
  box-sizing: border-box;
}

.back-button {
  background: #f6f8fa;
  border: 1px solid #d0d7de;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  width: 100%;
  transition: background 0.2s;
}

.back-button:hover {
  background: #e1e4e8;
}

.loading {
  padding: 16px;
  text-align: center;
  color: #57606a;
  font-size: 14px;
}

.file-tree {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
</style>
