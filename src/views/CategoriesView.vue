<script setup lang="ts">
import { computed, inject, onMounted, ref, watch, type Ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  addCategory,
  archiveCategory,
  getCategories,
  setCategoryHidden,
  updateCategory,
} from "../api";
import type { Category } from "../types";

const EMOJI_OPTIONS = [
  "🍜", "🚕", "🛒", "🏠", "🎮", "💊", "📚", "🎁", "📦",
  "☕", "🍰", "✈️", "🐶", "💄", "🏋️", "🧧", "🌷", "👶",
  "💰", "📈", "💵", "🪙", "💻", "🎉",
];

const categories = ref<Category[]>([]);
const kind = ref<0 | 1>(0);
const selectedMajorId = ref<number>();
const loading = ref(false);

const refreshKey = inject<Ref<number>>("refreshKey", ref(0));

const majors = computed(() =>
  categories.value
    .filter((c) => c.parentId === null && c.kind === kind.value)
    .sort((a, b) => a.sortOrder - b.sortOrder)
);

const selectedMajor = computed(() =>
  majors.value.find((m) => m.id === selectedMajorId.value)
);

const subs = computed(() =>
  categories.value
    .filter((c) => c.parentId === selectedMajorId.value)
    .sort((a, b) => a.sortOrder - b.sortOrder)
);

function subCount(majorId: number) {
  return categories.value.filter((c) => c.parentId === majorId).length;
}

// ---- 表单弹窗（新增/改名）----
interface FormState {
  mode: "addMajor" | "renameMajor" | "addSub" | "renameSub";
  target?: Category;
}
const formVisible = ref(false);
const formState = ref<FormState>({ mode: "addMajor" });
const formName = ref("");
const formIcon = ref(EMOJI_OPTIONS[0]);
const formSaving = ref(false);

const formTitle = computed(() => {
  switch (formState.value.mode) {
    case "addMajor":
      return "添加一级分类";
    case "renameMajor":
      return "修改一级分类";
    case "addSub":
      return `添加二级分类（${selectedMajor.value?.name ?? ""}）`;
    default:
      return "修改二级分类";
  }
});

const formShowIcon = computed(
  () => formState.value.mode === "addMajor" || formState.value.mode === "renameMajor"
);

function openForm(mode: FormState["mode"], target?: Category) {
  formState.value = { mode, target };
  formName.value = target?.name ?? "";
  formIcon.value = target?.icon || EMOJI_OPTIONS[0];
  formVisible.value = true;
}

async function submitForm() {
  if (!formName.value.trim() || formSaving.value) return;
  formSaving.value = true;
  try {
    const { mode, target } = formState.value;
    if (mode === "addMajor") {
      await addCategory({
        name: formName.value.trim(),
        icon: formIcon.value,
        kind: kind.value,
      });
      ElMessage.success("已添加一级分类");
    } else if (mode === "renameMajor" && target) {
      await updateCategory(target.id, {
        name: formName.value.trim(),
        icon: formIcon.value,
      });
      ElMessage.success("已修改");
    } else if (mode === "addSub" && selectedMajorId.value) {
      await addCategory({
        name: formName.value.trim(),
        parentId: selectedMajorId.value,
        kind: kind.value,
      });
      ElMessage.success("已添加二级分类");
    } else if (mode === "renameSub" && target) {
      await updateCategory(target.id, { name: formName.value.trim() });
      ElMessage.success("已修改");
    }
    formVisible.value = false;
    await load();
    refreshKey.value++;
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    formSaving.value = false;
  }
}

// ---- 隐藏 / 归档 ----
async function toggleHidden(cat: Category) {
  try {
    await setCategoryHidden(cat.id, !cat.hidden);
    ElMessage.success(cat.hidden ? "已显示" : "已隐藏");
    await load();
  } catch (err) {
    ElMessage.error(String(err));
  }
}

async function removeCategory(cat: Category) {
  const tip = cat.parentId === null
    ? `归档后「${cat.name}」及其所有二级分类将从记一笔的选择中消失；已有账单不受影响。`
    : `归档后「${cat.name}」将从记一笔的选择中消失；已有账单不受影响。`;
  try {
    await ElMessageBox.confirm(tip, "归档分类", {
      confirmButtonText: "归档",
      cancelButtonText: "取消",
      type: "warning",
    });
  } catch {
    return;
  }
  try {
    await archiveCategory(cat.id);
    ElMessage.success("已归档");
    if (selectedMajorId.value === cat.id) selectedMajorId.value = undefined;
    await load();
    refreshKey.value++;
  } catch (err) {
    ElMessage.error(String(err));
  }
}

async function load() {
  loading.value = true;
  try {
    categories.value = await getCategories();
    if (
      !selectedMajorId.value ||
      !majors.value.some((m) => m.id === selectedMajorId.value)
    ) {
      selectedMajorId.value = majors.value[0]?.id;
    }
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    loading.value = false;
  }
}
onMounted(load);
watch(kind, load);
</script>

<template>
  <div class="kind-toggle">
    <el-radio-group v-model="kind">
      <el-radio-button :value="0">支出分类</el-radio-button>
      <el-radio-button :value="1">收入分类</el-radio-button>
    </el-radio-group>
  </div>
  <div v-loading="loading" class="cat-layout">
    <!-- 左栏：一级大类 -->
    <div class="left-panel">
      <div class="panel-title">{{ kind === 0 ? "支出一级分类" : "收入一级分类" }}</div>
      <div
        v-for="m in majors"
        :key="m.id"
        class="major-item"
        :class="{ active: m.id === selectedMajorId, hidden: m.hidden }"
        @click="selectedMajorId = m.id"
      >
        <span class="major-icon">{{ m.icon }}</span>
        <span class="major-name">{{ m.name }}</span>
        <span v-if="m.hidden" class="hidden-badge">已隐藏</span>
        <span class="major-count">{{ subCount(m.id) }} 个</span>
      </div>
      <el-button class="add-btn" @click="openForm('addMajor')">
        ＋ 添加一级分类
      </el-button>
    </div>

    <!-- 右栏：选中大类的二级小类 -->
    <div class="right-panel">
      <template v-if="selectedMajor">
        <div class="major-header">
          <span class="major-icon big">{{ selectedMajor.icon }}</span>
          <span class="major-name">{{ selectedMajor.name }}</span>
          <span v-if="selectedMajor.hidden" class="hidden-badge">已隐藏</span>
          <div class="spacer"></div>
          <el-button link type="primary" @click="openForm('renameMajor', selectedMajor)">
            改名
          </el-button>
          <el-button link type="warning" @click="toggleHidden(selectedMajor)">
            {{ selectedMajor.hidden ? "显示" : "隐藏" }}
          </el-button>
          <el-button link type="danger" @click="removeCategory(selectedMajor)">
            归档
          </el-button>
        </div>
        <div class="sub-list">
          <div
            v-for="s in subs"
            :key="s.id"
            class="sub-item"
            :class="{ hidden: s.hidden }"
          >
            <span class="sub-name">{{ s.name }}</span>
            <span v-if="s.hidden" class="hidden-badge">已隐藏</span>
            <div class="spacer"></div>
            <el-button link type="primary" @click="openForm('renameSub', s)">
              改名
            </el-button>
            <el-button link type="warning" @click="toggleHidden(s)">
              {{ s.hidden ? "显示" : "隐藏" }}
            </el-button>
            <el-button link type="danger" @click="removeCategory(s)">
              归档
            </el-button>
          </div>
        </div>
        <el-button class="add-btn" @click="openForm('addSub')">
          ＋ 添加二级分类
        </el-button>
      </template>
      <el-empty
        v-else
        description="从左侧选择一个一级分类，或点击下方按钮添加"
      />
    </div>

    <!-- 新增/改名弹窗 -->
    <el-dialog
      v-model="formVisible"
      :title="formTitle"
      width="380px"
      :close-on-click-modal="false"
    >
      <div v-if="formShowIcon" class="field">
        <label>图标</label>
        <div class="emoji-grid">
          <button
            v-for="e in EMOJI_OPTIONS"
            :key="e"
            class="emoji-btn"
            :class="{ active: formIcon === e }"
            type="button"
            @click="formIcon = e"
          >
            {{ e }}
          </button>
        </div>
      </div>
      <div class="field">
        <label>名称</label>
        <el-input
          v-model="formName"
          maxlength="10"
          placeholder="分类名称（最多 10 个字）"
          @keyup.enter="submitForm"
        />
      </div>
      <template #footer>
        <el-button @click="formVisible = false">取消</el-button>
        <el-button
          type="primary"
          :disabled="!formName.trim()"
          :loading="formSaving"
          @click="submitForm"
        >
          保存
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.kind-toggle {
  margin-bottom: 16px;
}
.cat-layout {
  display: flex;
  gap: 20px;
  align-items: flex-start;
}
.left-panel {
  width: 260px;
  flex-shrink: 0;
  background: #fff;
  border-radius: 8px;
  padding: 12px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.panel-title {
  font-size: 13px;
  color: #909399;
  padding: 4px 8px 8px;
}
.major-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 2px;
}
.major-item:hover {
  background: #f5f7fa;
}
.major-item.active {
  background: #e4e1fc;
}
.major-item.hidden {
  opacity: 0.55;
}
.major-icon {
  font-size: 18px;
}
.major-icon.big {
  font-size: 26px;
}
.major-name {
  font-weight: 600;
  color: #303133;
}
.major-count {
  margin-left: auto;
  font-size: 12px;
  color: #909399;
}
.hidden-badge {
  font-size: 11px;
  color: #e6a23c;
  border: 1px solid #e6a23c;
  border-radius: 4px;
  padding: 0 4px;
}
.right-panel {
  flex: 1;
  min-width: 0;
  background: #fff;
  border-radius: 8px;
  padding: 16px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.major-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 12px;
  border-bottom: 1px solid #e4e7ed;
  margin-bottom: 8px;
}
.spacer {
  flex: 1;
}
.sub-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 4px;
  border-bottom: 1px solid #f0f2f5;
}
.sub-item.hidden {
  opacity: 0.55;
}
.sub-name {
  color: #303133;
}
.add-btn {
  width: 100%;
  margin-top: 12px;
}
.field {
  margin-bottom: 16px;
}
.field > label {
  display: block;
  margin-bottom: 6px;
  font-size: 13px;
  color: #606266;
}
.emoji-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.emoji-btn {
  width: 38px;
  height: 38px;
  font-size: 20px;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  background: #fff;
  cursor: pointer;
}
.emoji-btn:hover {
  border-color: #6c5ce7;
}
.emoji-btn.active {
  border-color: #6c5ce7;
  background: #e4e1fc;
}
</style>
