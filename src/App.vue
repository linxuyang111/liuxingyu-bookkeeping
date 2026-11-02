<script setup lang="ts">
import { provide, ref } from "vue";
import AddBillDialog from "./components/AddBillDialog.vue";

const dialogVisible = ref(false);
const refreshKey = ref(0);
provide("refreshKey", refreshKey);

function onSaved() {
  refreshKey.value++;
}
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <div class="logo">🌠 流星雨记账</div>
      <el-menu router :default-active="$route.path" class="menu">
        <el-menu-item index="/">📋 账单</el-menu-item>
        <el-menu-item index="/stats">📊 统计</el-menu-item>
        <el-menu-item index="/categories">🏷️ 分类</el-menu-item>
        <el-menu-item index="/settings">⚙️ 设置</el-menu-item>
      </el-menu>
    </aside>
    <main class="content">
      <router-view />
    </main>
    <button class="fab" title="记一笔" @click="dialogVisible = true">＋</button>
    <AddBillDialog v-model="dialogVisible" @saved="onSaved" />
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  height: 100vh;
}
.sidebar {
  width: 200px;
  background: #fff;
  border-right: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}
.logo {
  padding: 20px 16px;
  font-size: 18px;
  font-weight: 700;
  color: #303133;
}
.menu {
  border-right: none;
  flex: 1;
}
.content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}
.fab {
  position: fixed;
  right: 32px;
  bottom: 32px;
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  background: linear-gradient(135deg, #6c5ce7, #4dabf7);
  color: #fff;
  font-size: 28px;
  line-height: 1;
  box-shadow: 0 4px 12px rgba(108, 92, 231, 0.4);
  transition: transform 0.15s;
}
.fab:hover {
  transform: scale(1.08);
}
</style>
