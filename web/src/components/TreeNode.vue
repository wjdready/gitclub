<template>
  <div class="tree-node">
    <div
      class="node-content"
      :class="{ selected: isSelected, 'is-repo': node.is_repo }"
      @click="handleClick"
    >
      <span class="toggle" v-if="!node.is_repo && hasChildren" @click.stop="toggleExpand">
        {{ isExpanded ? '▼' : '▶' }}
      </span>
      <span class="icon">{{ node.is_repo ? '📦' : '📁' }}</span>
      <span class="name">{{ displayName }}</span>
    </div>
    <div v-if="isExpanded && hasChildren" class="children">
      <TreeNode
        v-for="child in node.children"
        :key="child.path"
        :node="child"
        :selected-path="selectedPath"
        :expanded-paths="expandedPaths"
        @select="$emit('select', $event)"
        @toggle="$emit('toggle', $event)"
      />
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  node: {
    type: Object,
    required: true
  },
  selectedPath: {
    type: String,
    default: ''
  },
  expandedPaths: {
    type: Set,
    required: true
  }
})

const emit = defineEmits(['select', 'toggle'])

const isSelected = computed(() => props.selectedPath === props.node.path)
const hasChildren = computed(() => props.node.children && props.node.children.length > 0)
const isExpanded = computed(() => props.expandedPaths.has(props.node.path))
const displayName = computed(() => {
  if (props.node.is_repo && props.node.name.endsWith('.git')) {
    return props.node.name.slice(0, -4)
  }
  return props.node.name
})

const toggleExpand = () => {
  emit('toggle', props.node.path)
}

const handleClick = () => {
  if (!props.node.is_repo && hasChildren.value) {
    toggleExpand()
  }
  emit('select', props.node)
}
</script>

<style scoped>
.tree-node {
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
  cursor: pointer;
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
</style>
