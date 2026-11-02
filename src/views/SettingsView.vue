<script setup lang="ts">
import { ref } from "vue";
import { ElMessage } from "element-plus";
import { save } from "@tauri-apps/plugin-dialog";
import dayjs from "dayjs";
import { exportExcel } from "../api";

const exporting = ref(false);

async function doExport() {
  if (exporting.value) return;
  exporting.value = true;
  try {
    const path = await save({
      title: "导出账单备份",
      defaultPath: `流星雨记账账单-${dayjs().format("YYYY-MM-DD")}.xlsx`,
      filters: [{ name: "Excel 表格", extensions: ["xlsx"] }],
    });
    if (!path) return; // 用户取消
    const count = await exportExcel(path);
    ElMessage.success(`已导出 ${count} 条账单`);
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div class="settings">
    <div class="card">
      <div class="card-title">💾 数据备份</div>
      <p class="card-desc">
        将全部账单导出为 Excel 文件，可以用表格软件查看，也可以作为备份保存。
        账单数据平时保存在本机数据库中，不会上传网络。
      </p>
      <el-button type="primary" :loading="exporting" @click="doExport">
        导出 Excel 账单
      </el-button>
    </div>

    <div class="card">
      <div class="card-title">🌠 关于流星雨记账</div>
      <p class="card-desc">
        版本 0.1.0 · Windows 桌面个人记账应用<br />
        本地存储 · 无需联网 · 数据只属于你
      </p>
    </div>
  </div>
</template>

<style scoped>
.settings {
  max-width: 560px;
}
.card {
  background: #fff;
  border-radius: 8px;
  padding: 20px 24px;
  margin-bottom: 16px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.card-title {
  font-weight: 600;
  color: #303133;
  margin-bottom: 10px;
}
.card-desc {
  color: #909399;
  font-size: 13px;
  line-height: 1.7;
  margin: 0 0 14px;
}
</style>
