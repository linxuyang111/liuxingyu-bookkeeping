import { invoke } from "@tauri-apps/api/core";
import type { AddBillInput, Bill, Category } from "./types";

export const getCategories = () => invoke<Category[]>("get_categories");
export const getBills = () => invoke<Bill[]>("get_bills");
export const addBill = (input: AddBillInput) => invoke<Bill>("add_bill", { input });
export const deleteBill = (id: number) => invoke<void>("delete_bill", { id });
