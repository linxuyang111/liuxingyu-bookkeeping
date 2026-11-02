<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as echarts from "echarts";
import dayjs from "dayjs";
import { getCategories, getMonthSummary, getTrend, getYearSummary } from "../api";
import type { Category, MonthSummary } from "../types";
import { fmtMoney, todayStr } from "../utils";

const categories = ref<Category[]>([]);
const summary = ref<MonthSummary | null>(null);
const trend = ref<{ month: string; totalCents: number }[]>([]);
const loading = ref(false);

const mode = ref<"month" | "year">("month");
const period = ref(todayStr().slice(0, 7)); // month: YYYY-MM, year: YYYY

const refreshKey = inject<Ref<number>>("refreshKey", ref(0));

const majors = computed(() =>
  categories.value.filter((c) => c.parentId === null)
);

const periodLabel = computed(() => {
  if (mode.value === "month") {
    const [y, m] = period.value.split("-").map(Number);
    return `${y}年${m}月`;
  }
  return `${period.value}年`;
});

// 分类专属色盘（按一级分类顺序取色，超出则循环）
const PALETTE = [
  "#f0932b", "#54a0ff", "#10ac84", "#6c5ce7", "#f368e0",
  "#eb4d4b", "#01a3a4", "#feca57", "#8395a7",
];
function colorOfMajor(majorId: number): string {
  const idx = majors.value.findIndex((m) => m.id === majorId);
  return PALETTE[idx >= 0 ? idx % PALETTE.length : 0];
}

const trendRef = ref<HTMLElement>();
const pieRef = ref<HTMLElement>();
let trendChart: echarts.ECharts | null = null;
let pieChart: echarts.ECharts | null = null;

function onResize() {
  trendChart?.resize();
  pieChart?.resize();
}

async function load() {
  loading.value = true;
  try {
    const [cats, sum, trendData] = await Promise.all([
      getCategories(),
      mode.value === "month"
        ? getMonthSummary(period.value)
        : getYearSummary(period.value),
      getTrend(12),
    ]);
    categories.value = cats;
    summary.value = sum;
    trend.value = fillTrend(trendData);
    renderCharts();
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    loading.value = false;
  }
}

/** 补全近 12 个月中缺失的月份（无账单月份记为 0） */
function fillTrend(points: { month: string; totalCents: number }[]) {
  const map = new Map(points.map((p) => [p.month, p.totalCents]));
  const result: { month: string; totalCents: number }[] = [];
  for (let i = 11; i >= 0; i--) {
    const m = dayjs().subtract(i, "month").format("YYYY-MM");
    result.push({ month: m, totalCents: map.get(m) ?? 0 });
  }
  return result;
}

function renderCharts() {
  if (trendChart && trendRef.value) {
    trendChart.setOption({
      tooltip: {
        trigger: "axis",
        valueFormatter: (v: unknown) =>
          "¥" + fmtMoney(Math.round(Number(v) * 100)),
      },
      grid: { left: 64, right: 16, top: 24, bottom: 28 },
      xAxis: {
        type: "category",
        data: trend.value.map((t) => dayjs(t.month + "-01").format("YY年M月")),
        axisTick: { show: false },
        axisLine: { lineStyle: { color: "#dcdfe6" } },
        axisLabel: { color: "#909399" },
      },
      yAxis: {
        type: "value",
        splitLine: { lineStyle: { color: "#f0f2f5" } },
        axisLabel: { color: "#909399", formatter: (v: number) => "¥" + v },
      },
      series: [
        {
          type: "bar",
          data: trend.value.map((t) => Math.round(t.totalCents) / 100),
          barWidth: 18,
          itemStyle: {
            borderRadius: [6, 6, 0, 0],
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: "#6c5ce7" },
              { offset: 1, color: "#4dabf7" },
            ]),
          },
        },
      ],
    });
  }
  if (pieChart && pieRef.value) {
    const data = (summary.value?.byMajor ?? []).map((m) => ({
      name: `${m.icon} ${m.name}`,
      value: m.totalCents / 100,
      itemStyle: { color: colorOfMajor(m.categoryId) },
    }));
    pieChart.setOption({
      tooltip: {
        trigger: "item",
        formatter: (p: { name: string; value: number; percent: number }) =>
          `${p.name}<br/>¥${fmtMoney(Math.round(p.value * 100))}（${p.percent}%）`,
      },
      legend: {
        bottom: 0,
        icon: "circle",
        itemWidth: 10,
        textStyle: { color: "#606266", fontSize: 12 },
      },
      series: [
        {
          type: "pie",
          radius: ["45%", "68%"],
          center: ["50%", "44%"],
          itemStyle: { borderRadius: 6, borderColor: "#fff", borderWidth: 2 },
          label: { show: false },
          emphasis: {
            label: { show: true, fontSize: 16, fontWeight: "bold", formatter: "{b}\n{d}%" },
          },
          data,
        },
      ],
    });
  }
}

onMounted(() => {
  trendChart = echarts.init(trendRef.value!);
  pieChart = echarts.init(pieRef.value!);
  window.addEventListener("resize", onResize);
  load();
});
onBeforeUnmount(() => {
  window.removeEventListener("resize", onResize);
  trendChart?.dispose();
  pieChart?.dispose();
});
watch([mode, period], load);
watch(refreshKey, load);

const hasBills = computed(() => (summary.value?.count ?? 0) > 0);
const maxTotal = computed(() =>
  Math.max(...(summary.value?.byMajor ?? []).map((m) => m.totalCents), 1)
);
</script>

<template>
  <div v-loading="loading">
    <!-- 支出趋势（近 12 个月） -->
    <div class="card">
      <div class="card-title">📈 支出趋势（近 12 个月）</div>
      <div v-show="trend.some((t) => t.totalCents > 0)" ref="trendRef" class="chart trend-chart"></div>
      <el-empty
        v-if="!trend.some((t) => t.totalCents > 0)"
        description="还没有账单数据，先记几笔再来看看吧"
      />
    </div>

    <div class="bottom-row">
      <!-- 分类占比 -->
      <div class="card half">
        <div class="card-title">
          🍩 分类占比
          <div class="period-control">
            <el-radio-group v-model="mode" size="small">
              <el-radio-button value="month">月</el-radio-button>
              <el-radio-button value="year">年</el-radio-button>
            </el-radio-group>
            <el-date-picker
              v-model="period"
              :type="mode === 'month' ? 'month' : 'year'"
              :value-format="mode === 'month' ? 'YYYY-MM' : 'YYYY'"
              :format="mode === 'month' ? 'YYYY年MM月' : 'YYYY年'"
              :clearable="false"
              size="small"
              class="period-picker"
            />
          </div>
        </div>
        <div v-show="hasBills" ref="pieRef" class="chart pie-chart"></div>
        <el-empty
          v-if="!hasBills"
          :description="`${periodLabel}没有账单数据`"
        />
      </div>

      <!-- 分类排行 -->
      <div class="card half">
        <div class="card-title">🏆 分类排行（{{ periodLabel }}）</div>
        <el-empty
          v-if="!hasBills"
          :description="`${periodLabel}没有账单数据`"
        />
        <div v-else class="rank-list">
          <div v-for="(m, i) in summary?.byMajor" :key="m.categoryId" class="rank-row">
            <span class="rank-no" :class="{ top: i < 3 }">{{ i + 1 }}</span>
            <span class="rank-icon">{{ m.icon }}</span>
            <div class="rank-main">
              <div class="rank-top">
                <span class="rank-name">{{ m.name }}</span>
                <b class="rank-amount">¥{{ fmtMoney(m.totalCents) }}</b>
              </div>
              <div class="rank-bar">
                <div
                  class="rank-bar-fill"
                  :style="{
                    width: (m.totalCents / maxTotal) * 100 + '%',
                    background: colorOfMajor(m.categoryId),
                  }"
                ></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  background: #fff;
  border-radius: 8px;
  padding: 16px 20px;
  margin-bottom: 16px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.card-title {
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
}
.bottom-row {
  display: flex;
  gap: 16px;
  align-items: stretch;
}
.half {
  flex: 1;
  min-width: 0;
  margin-bottom: 0;
}
.chart {
  width: 100%;
}
.trend-chart {
  height: 280px;
}
.pie-chart {
  height: 300px;
}
.period-control {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}
.period-picker {
  width: 120px;
}
.rank-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.rank-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.rank-no {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #f0f2f5;
  color: #909399;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.rank-no.top {
  background: #6c5ce7;
  color: #fff;
}
.rank-icon {
  font-size: 18px;
}
.rank-main {
  flex: 1;
  min-width: 0;
}
.rank-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}
.rank-name {
  color: #303133;
  font-size: 13px;
}
.rank-amount {
  color: #6c5ce7;
  font-size: 13px;
}
.rank-bar {
  height: 6px;
  background: #f0f2f5;
  border-radius: 3px;
  overflow: hidden;
}
.rank-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s;
}
</style>
