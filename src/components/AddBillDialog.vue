<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import { addBill, getCategories } from "../api";
import type { Category } from "../types";
import { todayStr } from "../utils";

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{
  (e: "update:modelValue", v: boolean): void;
  (e: "saved"): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (v) => emit("update:modelValue", v),
});

const categories = ref<Category[]>([]);
const amount = ref<number>();
const majorId = ref<number>();
const subId = ref<number>();
const billDate = ref(todayStr());
const note = ref("");
const saving = ref(false);

const majors = computed(() =>
  categories.value.filter((c) => c.parentId === null && !c.hidden)
);
const subs = computed(() =>
  categories.value.filter((c) => c.parentId === majorId.value && !c.hidden)
);

watch(majorId, (id) => {
  const list = categories.value.filter((c) => c.parentId === id && !c.hidden);
  subId.value = list.length === 1 ? list[0].id : undefined;
});

async function loadCategories() {
  categories.value = await getCategories();
  if (!majorId.value && majors.value.length > 0) {
    majorId.value = majors.value[0].id;
  }
}
onMounted(loadCategories);

const canSave = computed(
  () => !!amount.value && amount.value > 0 && !!subId.value
);

async function save() {
  if (!canSave.value || saving.value) return;
  saving.value = true;
  try {
    await addBill({
      amountCents: Math.round(amount.value! * 100),
      categoryId: subId.value!,
      billDate: billDate.value,
      note: note.value.trim(),
    });
    ElMessage.success("已记账");
    emit("saved");
    visible.value = false;
    amount.value = undefined;
    note.value = "";
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    saving.value = false;
  }
}

function close() {
  visible.value = false;
}
</script>

<template>
  <el-dialog
    v-model="visible"
    title="记一笔"
    width="420px"
    :close-on-click-modal="false"
  >
    <div class="field">
      <label>金额（元）</label>
      <el-input-number
        v-model="amount"
        :min="0.01"
        :precision="2"
        :controls="false"
        placeholder="0.00"
        class="amount-input"
        @keyup.enter="save"
      />
    </div>
    <div class="field">
      <label>分类</label>
      <div class="category-row">
        <el-select v-model="majorId" placeholder="一级分类" class="cat-select">
          <el-option
            v-for="m in majors"
            :key="m.id"
            :label="`${m.icon} ${m.name}`"
            :value="m.id"
          />
        </el-select>
        <el-select v-model="subId" placeholder="二级分类" class="cat-select">
          <el-option
            v-for="s in subs"
            :key="s.id"
            :label="s.name"
            :value="s.id"
          />
        </el-select>
      </div>
    </div>
    <div class="field">
      <label>日期</label>
      <el-date-picker
        v-model="billDate"
        type="date"
        value-format="YYYY-MM-DD"
        :clearable="false"
        class="date-input"
      />
    </div>
    <div class="field">
      <label>备注</label>
      <el-input
        v-model="note"
        type="textarea"
        :rows="2"
        maxlength="200"
        show-word-limit
        placeholder="备注（可选）"
      />
    </div>
    <template #footer>
      <el-button @click="close">取消</el-button>
      <el-button type="primary" :disabled="!canSave" :loading="saving" @click="save">
        保存
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.field {
  margin-bottom: 16px;
}
.field > label {
  display: block;
  margin-bottom: 6px;
  font-size: 13px;
  color: #606266;
}
.amount-input {
  width: 100%;
}
.amount-input :deep(.el-input__inner) {
  font-size: 24px;
  font-weight: 700;
  height: 48px;
}
.category-row {
  display: flex;
  gap: 12px;
}
.cat-select {
  flex: 1;
}
.date-input {
  width: 100%;
}
</style>
