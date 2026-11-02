<script setup lang="ts">
import { computed, inject, onMounted, ref, watch, type Ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import dayjs from "dayjs";
import { deleteBill, getBills, getCategories, getMonthSummary } from "../api";
import type { Bill, Category, MonthSummary } from "../types";
import { dayLabel, fmtMoney, todayStr } from "../utils";

const bills = ref<Bill[]>([]);
const categories = ref<Category[]>([]);
const summary = ref<MonthSummary | null>(null);
const loading = ref(false);

const month = ref(todayStr().slice(0, 7));
const filterCategoryId = ref<number>();
const keyword = ref("");
let keywordTimer: ReturnType<typeof setTimeout> | undefined;

const refreshKey = inject<Ref<number>>("refreshKey", ref(0));
const openEdit = inject<(bill: Bill) => void>("openEdit");

const majors = computed(() =>
  categories.value.filter((c) => c.parentId === null && !c.hidden)
);

const monthLabel = computed(() => {
  const [y, m] = month.value.split("-").map(Number);
  return `${y}年${m}月`;
});

const avgPerDay = computed(() => {
  if (!summary.value || summary.value.totalCents === 0) return 0;
  let divisor: number;
  if (month.value === todayStr().slice(0, 7)) {
    divisor = dayjs().date(); // 本月已过天数
  } else {
    divisor = dayjs(month.value + "-01").daysInMonth();
  }
  return Math.round(summary.value.totalCents / Math.max(divisor, 1));
});

const topMajors = computed(() => (summary.value?.byMajor ?? []).slice(0, 5));

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
    const [billList, catList, sum] = await Promise.all([
      getBills({
        month: month.value,
        categoryId: filterCategoryId.value,
        keyword: keyword.value.trim() || undefined,
      }),
      getCategories(),
      getMonthSummary(month.value),
    ]);
    bills.value = billList;
    categories.value = catList;
    summary.value = sum;
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    loading.value = false;
  }
}

onMounted(load);
watch([month, filterCategoryId], load);
watch(refreshKey, load);
watch(keyword, () => {
  clearTimeout(keywordTimer);
  keywordTimer = setTimeout(load, 300);
});

function shiftMonth(delta: number) {
  month.value = dayjs(month.value + "-01")
    .add(delta, "month")
    .format("YYYY-MM");
}

const isCurrentMonth = computed(() => month.value === todayStr().slice(0, 7));

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

function edit(bill: Bill) {
  if (openEdit) openEdit(bill);
}
</script>

<template>
  <div v-loading="loading">
    <!-- 月度汇总卡片 -->
    <div class="summary-row">
      <div class="stat-card stat-main">
        <div class="stat-label">{{ monthLabel }}总支出</div>
        <div class="stat-value stat-value-big">
          ¥{{ fmtMoney(summary?.totalCents ?? 0) }}
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-label">日均支出</div>
        <div class="stat-value">¥{{ fmtMoney(avgPerDay) }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">记账笔数</div>
        <div class="stat-value">{{ summary?.count ?? 0 }} 笔</div>
      </div>
    </div>

    <!-- 分类小计排行 -->
    <div v-if="topMajors.length > 0" class="major-pills">
      <span
        v-for="m in topMajors"
        :key="m.categoryId"
        class="major-pill"
      >
        {{ m.icon }} {{ m.name }}
        <b>¥{{ fmtMoney(m.totalCents) }}</b>
      </span>
    </div>

    <!-- 筛选栏 -->
    <div class="filter-bar">
      <el-button-group class="month-nav">
        <el-button @click="shiftMonth(-1)">◀</el-button>
        <el-date-picker
          v-model="month"
          type="month"
          value-format="YYYY-MM"
          format="YYYY年MM月"
          :clearable="false"
          class="month-picker"
        />
        <el-button :disabled="isCurrentMonth" @click="shiftMonth(1)">▶</el-button>
      </el-button-group>
      <el-select
        v-model="filterCategoryId"
        placeholder="全部分类"
        clearable
        class="cat-filter"
      >
        <el-option
          v-for="m in majors"
          :key="m.id"
          :label="`${m.icon} ${m.name}`"
          :value="m.id"
        />
      </el-select>
      <el-input
        v-model="keyword"
        placeholder="搜索备注"
        clearable
        class="keyword-input"
      />
    </div>

    <!-- 账单列表 -->
    <el-empty
      v-if="groups.length === 0"
      description="这个月还没有账单，点右下角 ＋ 记一笔吧"
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
        <el-button link type="primary" class="bill-action" @click="edit(b)">
          编辑
        </el-button>
        <el-button link type="danger" class="bill-action" @click="remove(b)">
          删除
        </el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.summary-row {
  display: flex;
  gap: 16px;
  margin-bottom: 12px;
}
.stat-card {
  flex: 1;
  background: #fff;
  border-radius: 8px;
  padding: 16px 20px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.stat-main {
  background: linear-gradient(135deg, #6c5ce7, #4dabf7);
}
.stat-main .stat-label,
.stat-main .stat-value {
  color: #fff;
}
.stat-label {
  font-size: 13px;
  color: #909399;
  margin-bottom: 6px;
}
.stat-value {
  font-size: 22px;
  font-weight: 700;
  color: #303133;
}
.stat-value-big {
  font-size: 28px;
}
.major-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 16px;
}
.major-pill {
  background: #fff;
  border-radius: 16px;
  padding: 6px 14px;
  font-size: 13px;
  color: #606266;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.major-pill b {
  color: #6c5ce7;
  margin-left: 4px;
}
.filter-bar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}
.month-nav {
  display: flex;
  align-items: center;
}
.month-picker {
  width: 130px;
}
.cat-filter {
  width: 160px;
}
.keyword-input {
  width: 200px;
}
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
