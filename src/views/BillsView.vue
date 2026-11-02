<script setup lang="ts">
import { computed, inject, onMounted, ref, watch, type Ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { deleteBill, getBills } from "../api";
import type { Bill } from "../types";
import { dayLabel, fmtMoney } from "../utils";

const bills = ref<Bill[]>([]);
const loading = ref(false);

const refreshKey = inject<Ref<number>>("refreshKey", ref(0));

interface DayGroup {
  date: string;
  label: string;
  total: number;
  bills: Bill[];
}

const groups = computed<DayGroup[]>(() => {
  const map = new Map<string, DayGroup>();
  for (const b of bills.value) {
    let g = map.get(b.billDate);
    if (!g) {
      g = { date: b.billDate, label: dayLabel(b.billDate), total: 0, bills: [] };
      map.set(b.billDate, g);
    }
    g.total += b.amountCents;
    g.bills.push(b);
  }
  return [...map.values()];
});

async function load() {
  loading.value = true;
  try {
    bills.value = await getBills();
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    loading.value = false;
  }
}
onMounted(load);
watch(refreshKey, load);

async function remove(bill: Bill) {
  try {
    await ElMessageBox.confirm(
      `确定删除这笔「${bill.categoryName}」¥${fmtMoney(bill.amountCents)} 的账单吗？`,
      "删除账单",
      { confirmButtonText: "删除", cancelButtonText: "取消", type: "warning" }
    );
  } catch {
    return; // 用户取消
  }
  try {
    await deleteBill(bill.id);
    ElMessage.success("已删除");
    load();
  } catch (err) {
    ElMessage.error(String(err));
  }
}
</script>

<template>
  <div v-loading="loading">
    <el-empty
      v-if="groups.length === 0"
      description="还没有账单，点右下角 ＋ 记一笔吧"
    />
    <div v-for="g in groups" :key="g.date" class="day-group">
      <div class="day-header">
        <span class="day-label">{{ g.label }}</span>
        <span class="day-total">支出 ¥{{ fmtMoney(g.total) }}</span>
      </div>
      <div v-for="b in g.bills" :key="b.id" class="bill-item">
        <span class="bill-icon">{{ b.categoryIcon }}</span>
        <div class="bill-info">
          <div class="bill-category">
            {{ b.parentName ? `${b.parentName} / ${b.categoryName}` : b.categoryName }}
          </div>
          <div v-if="b.note" class="bill-note">{{ b.note }}</div>
        </div>
        <span class="bill-amount">-¥{{ fmtMoney(b.amountCents) }}</span>
        <el-button link type="danger" class="bill-delete" @click="remove(b)">
          删除
        </el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.day-group {
  margin-bottom: 20px;
}
.day-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 4px;
}
.day-label {
  font-weight: 600;
  color: #303133;
}
.day-total {
  font-size: 13px;
  color: #909399;
}
.bill-item {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #fff;
  border-radius: 8px;
  padding: 12px 16px;
  margin-bottom: 8px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.bill-icon {
  font-size: 22px;
}
.bill-info {
  flex: 1;
  min-width: 0;
}
.bill-category {
  color: #303133;
}
.bill-note {
  color: #909399;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bill-amount {
  font-weight: 700;
  color: #f56c6c;
}
</style>
