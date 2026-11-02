import { invoke } from "@tauri-apps/api/core";
import type {
  AddBillInput,
  AddCategoryInput,
  Bill,
  BillFilters,
  Category,
  MonthSummary,
  UpdateCategoryInput,
} from "./types";

export const getCategories = () => invoke<Category[]>("get_categories");
export const getBills = (filters: BillFilters) =>
  invoke<Bill[]>("get_bills", { filters });
export const addBill = (input: AddBillInput) =>
  invoke<Bill>("add_bill", { input });
export const updateBill = (id: number, input: AddBillInput) =>
  invoke<Bill>("update_bill", { id, input });
export const deleteBill = (id: number) => invoke<void>("delete_bill", { id });

export const addCategory = (input: AddCategoryInput) =>
  invoke<Category>("add_category", { input });
export const updateCategory = (id: number, input: UpdateCategoryInput) =>
  invoke<void>("update_category", { id, input });
export const setCategoryHidden = (id: number, hidden: boolean) =>
  invoke<void>("set_category_hidden", { id, hidden });
export const archiveCategory = (id: number) =>
  invoke<void>("archive_category", { id });

export const getMonthSummary = (month: string) =>
  invoke<MonthSummary>("get_month_summary", { month });
