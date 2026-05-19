<template>
  <div class="app">
    <header class="header">
      <h1>GitClub</h1>
      <p class="subtitle">Git 托管服务器</p>
    </header>

    <main class="main">
      <div class="card">
        <h2>服务器状态</h2>
        <div class="status">
          <span class="status-indicator" :class="{ online: isOnline }"></span>
          <span>{{ isOnline ? '在线' : '离线' }}</span>
        </div>
      </div>

      <div class="card" v-if="serverInfo">
        <h2>服务器信息</h2>
        <div class="info">
          <p><strong>消息:</strong> {{ serverInfo.message }}</p>
          <p><strong>版本:</strong> {{ serverInfo.version }}</p>
        </div>
      </div>

      <div class="card">
        <h2>快速开始</h2>
        <div class="quick-start">
          <p>欢迎使用 GitClub！这是一个基于 Rust 和 Vue 的 Git 托管服务器。</p>
          <ul>
            <li>支持群组和子组的层级管理</li>
            <li>灵活的用户权限控制</li>
            <li>支持 HTTP 和 SSH 协议</li>
          </ul>
        </div>
      </div>
    </main>

    <footer class="footer">
      <p>&copy; 2026 GitClub. All rights reserved.</p>
    </footer>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const isOnline = ref(false)
const serverInfo = ref(null)

const checkHealth = async () => {
  try {
    const response = await fetch('/api/health')
    isOnline.value = response.ok
  } catch (error) {
    console.error('健康检查失败:', error)
    isOnline.value = false
  }
}

const fetchServerInfo = async () => {
  try {
    const response = await fetch('/api/info')
    if (response.ok) {
      serverInfo.value = await response.json()
    }
  } catch (error) {
    console.error('获取服务器信息失败:', error)
  }
}

onMounted(() => {
  checkHealth()
  fetchServerInfo()
})
</script>

<style scoped>
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 2rem;
  text-align: center;
}

.header h1 {
  margin: 0;
  font-size: 2.5rem;
}

.subtitle {
  margin: 0.5rem 0 0 0;
  opacity: 0.9;
}

.main {
  flex: 1;
  max-width: 1200px;
  width: 100%;
  margin: 2rem auto;
  padding: 0 1rem;
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
}

.card {
  background: white;
  border-radius: 8px;
  padding: 1.5rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.card h2 {
  margin-top: 0;
  color: #333;
  font-size: 1.25rem;
}

.status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 1.1rem;
}

.status-indicator {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #e74c3c;
}

.status-indicator.online {
  background: #2ecc71;
}

.info p {
  margin: 0.5rem 0;
}

.quick-start ul {
  margin: 1rem 0;
  padding-left: 1.5rem;
}

.quick-start li {
  margin: 0.5rem 0;
}

.footer {
  background: #f8f9fa;
  padding: 1rem;
  text-align: center;
  color: #666;
  margin-top: auto;
}

.footer p {
  margin: 0;
}
</style>
