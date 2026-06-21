<template>
  <div class="main-content">
    <h1>CapPix</h1>
    <p>按 Ctrl+Shift+A 截图，或右键托盘图标</p>
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
.main-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  color: #e5e7eb;
}

h1 {
  font-size: 2rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

p {
  color: #9ca3af;
}
</style>
