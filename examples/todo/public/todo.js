let doc = (globalThis).document;
let storage = (globalThis).localStorage;
const STORAGE_KEY = "draconic-todo";
let todos = [];
let nextId = 1;
let filter = "all";
function loadTodos() {
let raw = (storage).getItem(STORAGE_KEY);
if (((raw) === (null)) || ((raw) === (""))) {
(todos = []);
(nextId = 1);
return;
}
let parsed = (JSON).parse(raw);
if (((parsed) === (null)) || ((typeof ((parsed).length)) !== ("number"))) {
(todos = []);
(nextId = 1);
return;
}
let loaded = [];
let maxId = 0;
for (let i = 0; (i) < ((parsed).length); (i = (i) + (1))) {
let item = (parsed)[i];
let t = {id: (item).id, text: (item).text, done: ((item).done) === (true)};
(loaded).push(t);
if (((t).id) > (maxId)) {
(maxId = (t).id);
}
}
(todos = loaded);
(nextId = (maxId) + (1));
}
function saveTodos() {
(storage).setItem(STORAGE_KEY, (JSON).stringify(todos));
}
function remainingCount() {
let n = 0;
for (let i = 0; (i) < ((todos).length); (i = (i) + (1))) {
if (!(((todos)[i]).done)) {
(n = (n) + (1));
}
}
return n;
}
function visibleTodos() {
let out = [];
for (let i = 0; (i) < ((todos).length); (i = (i) + (1))) {
let t = (todos)[i];
if ((filter) === ("all")) {
(out).push(t);
}
 else {
if (((filter) === ("active")) && (!((t).done))) {
(out).push(t);
}
 else {
if (((filter) === ("completed")) && ((t).done)) {
(out).push(t);
}
}
}
}
return out;
}
function addTodo(text) {
let trimmed = (text).trim();
if ((trimmed) === ("")) {
return;
}
let t = {id: nextId, text: trimmed, done: false};
(nextId = (nextId) + (1));
(todos).push(t);
(saveTodos)();
(render)();
}
function toggleTodo(id) {
for (let i = 0; (i) < ((todos).length); (i = (i) + (1))) {
if ((((todos)[i]).id) === (id)) {
(((todos)[i]).done = !(((todos)[i]).done));
(saveTodos)();
(render)();
return;
}
}
}
function deleteTodo(id) {
let next = [];
for (let i = 0; (i) < ((todos).length); (i = (i) + (1))) {
if ((((todos)[i]).id) !== (id)) {
(next).push((todos)[i]);
}
}
(todos = next);
(saveTodos)();
(render)();
}
function clearCompleted() {
let next = [];
for (let i = 0; (i) < ((todos).length); (i = (i) + (1))) {
if (!(((todos)[i]).done)) {
(next).push((todos)[i]);
}
}
(todos = next);
(saveTodos)();
(render)();
}
function setFilter(value) {
(filter = value);
(render)();
}
function renderTodoItem(t) {
let li = (doc).createElement("li");
((li).className = ((t).done) ? ("todo completed") : ("todo"));
(li).setAttribute("data-id", (String)((t).id));
let label = (doc).createElement("label");
((label).className = "todo-label");
let checkbox = (doc).createElement("input");
((checkbox).type = "checkbox");
((checkbox).checked = (t).done);
(checkbox).addEventListener("change", () => {
(toggleTodo)((t).id);
});
let span = (doc).createElement("span");
((span).className = "todo-text");
((span).textContent = (t).text);
(label).appendChild(checkbox);
(label).appendChild(span);
let del = (doc).createElement("button");
((del).type = "button");
((del).className = "todo-delete");
((del).textContent = "Delete");
(del).addEventListener("click", () => {
(deleteTodo)((t).id);
});
(li).appendChild(label);
(li).appendChild(del);
return li;
}
function render() {
let list = (doc).getElementById("todo-list");
let countEl = (doc).getElementById("todo-count");
let footer = (doc).getElementById("todo-footer");
let filters = (doc).getElementById("filters");
((list).innerHTML = "");
let visible = (visibleTodos)();
for (let i = 0; (i) < ((visible).length); (i = (i) + (1))) {
(list).appendChild((renderTodoItem)((visible)[i]));
}
let left = (remainingCount)();
if ((left) === (1)) {
((countEl).textContent = "1 item left");
}
 else {
((countEl).textContent = ((String)(left)) + (" items left"));
}
(((footer).style).display = (((todos).length) === (0)) ? ("none") : (""));
let buttons = (filters).querySelectorAll("button");
for (let i = 0; (i) < ((buttons).length); (i = (i) + (1))) {
let btn = (buttons)[i];
let f = (btn).getAttribute("data-filter");
if ((f) === (filter)) {
((btn).className = "filter active");
}
 else {
((btn).className = "filter");
}
}
}
function wireEvents() {
let form = (doc).getElementById("todo-form");
let input = (doc).getElementById("todo-input");
let filters = (doc).getElementById("filters");
let clearBtn = (doc).getElementById("clear-completed");
(form).addEventListener("submit", (e) => {
(e).preventDefault();
(addTodo)((input).value);
((input).value = "");
(input).focus();
});
(filters).addEventListener("click", (e) => {
let target = (e).target;
if ((target) === (null)) {
return;
}
let f = (target).getAttribute("data-filter");
if (((f) === (null)) || ((f) === (""))) {
return;
}
(setFilter)(f);
});
(clearBtn).addEventListener("click", () => {
(clearCompleted)();
});
}
function boot() {
(loadTodos)();
(wireEvents)();
(render)();
}
if (((doc).readyState) === ("loading")) {
(doc).addEventListener("DOMContentLoaded", () => {
(boot)();
});
}
 else {
(boot)();
}
