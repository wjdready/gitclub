<template>
  <div class="file-node">
    <div
      class="node-content"
      :class="{ selected: isSelected, 'is-file': !item.is_dir }"
      @click="handleClick"
    >
      <span class="toggle" v-if="item.is_dir" @click.stop="toggleExpand">
        {{ isExpanded ? '▼' : '▶' }}
      </span>
      <span class="icon">{{ item.is_dir ? '📁' : '📄' }}</span>
      <span class="name">{{ item.name }}</span>
    </div>
    <div v-if="isExpanded && item.is_dir" class="children">
      <div v-if="loading" class="loading-children">Loading...</div>
      <FileBrowserNode
        v-else
        v-for="child in children"
        :key="child.path"
        :item="child"
        :current-path="currentPath"
        :repo-path="repoPath"
        :branch="branch"
        :expanded-file-paths="expandedFilePaths"
        @navigate="$emit('navigate', $event)"
        @toggle="$emit('toggle', $event)"
      />
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'

const props = defineProps({
  item: {
    type: Object,
    required: true
  },
  currentPath: {
    type: String,
    default: ''
  },
  repoPath: {
    type: String,
    required: true
  },
  branch: {
    type: String,
    default: ''
  },
  expandedFilePaths: {
    type: Set,
    required: true
  }
})

const emit = defineEmits(['navigate', 'toggle'])

const children = ref([])
const loading = ref(false)

const isSelected = computed(() => props.currentPath === props.item.path)
const isExpanded = computed(() => props.expandedFilePaths.has(props.item.path))

const loadChildren = async () => {
  if (children.value.length > 0) return

  loading.value = true
  try {
    const url = `/api/repo/${props.repoPath}?path=${encodeURIComponent(props.item.path)}${props.branch ? '&branch=' + props.branch : ''}`
    const response = await fetch(url)
    const data = await response.json()
    children.value = data.files.map(file => ({
      name: file.name,
      path: file.path,
      is_dir: file.is_dir
    }))
  } catch (error) {
    console.error('Failed to load children:', error)
  } finally {
    loading.value = false
  }
}

const toggleExpand = async () => {
  if (!isExpanded.value && props.item.is_dir) {
    await loadChildren()
  }
  emit('toggle', props.item.path)
}

const handleClick = async () => {
  if (props.item.is_dir) {
    await toggleExpand()
    emit('navigate', props.item.path)
  }
}

// 当节点被自动展开时，自动加载子节点
watch(isExpanded, async (newValue) => {
  if (newValue && props.item.is_dir && children.value.length === 0) {
    await loadChildren()
  }
}, { immediate: true })
</script>

<style scoped>
.file-node {
  user-select: none;
}

.node-content {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s;
}

.node-content:hover {
  background: #f6f8fa;
}

.node-content.selected {
  background: #ddf4ff;
  color: #0969da;
}

.toggle {
  width: 16px;
  font-size: 10px;
  color: #57606a;
}

.icon {
  font-size: 16px;
}

.name {
  font-size: 14px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.children {
  margin-left: 20px;
  border-left: 1px solid #d0d7de;
  padding-left: 4px;
}

.loading-children {
  padding: 8px 16px;
  font-size: 12px;
  color: #57606a;
}
</style>
