const $=s=>document.querySelector(s);
const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));

/* ========== SVG 图标 ========== */
const ICONS={
  dashboard:'<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
  file:'<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M9 13h6M9 17h6"/>',
  folder:'<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  chart:'<path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/>',
  shield:'<path d="M12 3l8 3v6c0 4.5-3.2 8-8 9-4.8-1-8-4.5-8-9V6z"/><path d="M9 12l2 2 4-4"/>',
  home:'<path d="M3 11l9-8 9 8"/><path d="M5 10v10h14V10"/>',
  logout:'<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="M16 17l5-5-5-5M21 12H9"/>',
  download:'<path d="M12 3v12M7 10l5 5 5-5M4 21h16"/>',
  copy:'<rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/>',
  alert:'<path d="M12 3l10 18H2z"/><path d="M12 10v5M12 18v.01"/>',
  clock:'<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  check:'<circle cx="12" cy="12" r="9"/><path d="M8 12l3 3 5-6"/>'
};
function ic(name){
  return `<svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${ICONS[name]||''}</svg>`;
}
function applyIcons(root=document){
  root.querySelectorAll('[data-icon]').forEach(e=>{e.innerHTML=ic(e.dataset.icon);});
}

/* ========== 多语言字典 ========== */
const I18N={
'zh-CN':{
site_title:'文件分享',admin_title:'管理后台',back_home:'返回首页',admin_link:'管理',
pg_home_t:'文件分享',pg_home_d:'搜索、复制地址,或直接下载你需要的文件',
search_ph:'搜索名字或说明...',th_req:'要求',th_name:'名字',th_addr:'地址(可复制)',th_desc:'说明',th_dl:'下载',
req:'必须',opt:'可选',copy:'复制',copied:'已复制',all:'全部',none_found:'没有找到内容',
prev:'上一页',next:'下一页',page_info:'第 {0} / {1} 页,共 {2} 条',dead:'失效',
ui_native:'原生深色',ui_saas:'SaaS 浅色',ui_tip:'界面风格',accent_tip:'主题色',lang_tip:'语言',role_admin:'管理员',
pg_login_t:'欢迎回来',pg_login_d:'登录后管理你的文件分享站',
login:'登录',username:'用户名',password:'密码',logout:'退出登录',
nav_dashboard:'仪表盘',tab_items:'条目管理',tab_cats:'分类管理',tab_stats:'统计',tab_sec:'安全',
pg_dash_t:'仪表盘',pg_dash_d:'一眼查看站点概况和待处理事项',
pg_items_t:'条目管理',pg_items_d:'新增、编辑、上传、排序和批量处理文件条目',
pg_cats_t:'分类管理',pg_cats_d:'用分类整理条目,方便访客筛选',
pg_stats_t:'统计',pg_stats_d:'近 30 天的下载、复制趋势和访问来源',
pg_sec_t:'安全',pg_sec_d:'修改登录密码,管理 IP 黑名单',
st_items:'条目总数',st_cats:'分类数量',st_dl_total:'累计下载',st_dl_today:'今日下载',st_cp_today:'今日复制',st_dead:'失效链接',
remind_title:'提醒事项',rm_dead:'链接失效:{0}',rm_unchecked:'有 {0} 个条目尚未检测',rm_uncat:'有 {0} 个条目未分类',
rm_ok:'一切正常,没有待处理事项',rm_check:'立即检测',rm_view:'处理',rm_checked:'检测于 {0}',rm_never:'从未检测',
f_name:'名字 *',f_url:'地址 * (http/https/rtmp...)',f_desc:'说明',no_cat:'(无分类)',sort:'排序',sort_tip:'排序,越小越靠前',
add:'新增',save:'保存修改',cancel:'取消编辑',upload:'上传文件',uploading:'上传中...',
search:'搜索',all_cats:'全部分类',edit:'编辑',del:'删除',status:'状态',ok:'正常',unknown:'未检测',
th_cat:'分类',th_sort:'排序',th_copies:'复制',th_ops:'操作',
b_del:'批量删除',b_move:'移动到分类',drag_tip:'拖动行可调整顺序(仅在未搜索、未筛选时生效)',
check_now:'立即检测失效链接',check_started:'已开始后台检测,稍后刷新查看结果',
confirm_del:'确定删除该条目?',confirm_del_n:'确定删除所选 {0} 项?',confirm_cat_del:'删除分类后,其下条目会变为"无分类",继续?',
cat_name_ph:'分类名称',cat_count:'条目数',prompt_name:'分类名称',prompt_sort:'排序(越小越靠前)',
stats_trend:'近 30 天趋势',downloads:'下载',copies:'复制',stats_sources:'访问来源',stats_top:'热门条目',no_data:'暂无数据',
source:'来源',times:'次数',
old_pw:'旧密码',new_pw:'新密码(至少 8 位)',change_pw:'修改密码',pw_ok:'密码已修改,其他设备已退出登录',
bl_title:'IP 黑名单',ip_ph:'IP 地址',note_ph:'备注',bl_add:'加入黑名单',unblock:'解除',th_time:'时间',
site_settings:'站点设置',name_dl_sw:'允许点击名字下载(跳转)',save_ok:'已保存',
err_400:'参数错误(地址需以 http/https 等开头,仅限 ASCII,不含空格)',err_401:'用户名或密码错误',err_401s:'登录已过期,请重新登录',
err_403:'访问被拒绝',err_404:'不存在',err_409:'已存在(地址、分类名重复或检测进行中)',err_422:'旧密码错误',
err_429:'尝试次数过多,请 15 分钟后再试',err_502:'对象存储上传失败',err_x:'错误 {0}'
},
'zh-TW':{
site_title:'檔案分享',admin_title:'管理後台',back_home:'返回首頁',admin_link:'管理',
pg_home_t:'檔案分享',pg_home_d:'搜尋、複製位址,或直接下載你需要的檔案',
search_ph:'搜尋名稱或說明...',th_req:'要求',th_name:'名稱',th_addr:'位址(可複製)',th_desc:'說明',th_dl:'下載',
req:'必須',opt:'可選',copy:'複製',copied:'已複製',all:'全部',none_found:'找不到內容',
prev:'上一頁',next:'下一頁',page_info:'第 {0} / {1} 頁,共 {2} 筆',dead:'失效',
ui_native:'原生深色',ui_saas:'SaaS 淺色',ui_tip:'介面風格',accent_tip:'主題色',lang_tip:'語言',role_admin:'管理員',
pg_login_t:'歡迎回來',pg_login_d:'登入後管理你的檔案分享站',
login:'登入',username:'使用者名稱',password:'密碼',logout:'登出',
nav_dashboard:'儀表板',tab_items:'項目管理',tab_cats:'分類管理',tab_stats:'統計',tab_sec:'安全',
pg_dash_t:'儀表板',pg_dash_d:'一眼查看站點概況和待處理事項',
pg_items_t:'項目管理',pg_items_d:'新增、編輯、上傳、排序和批量處理檔案項目',
pg_cats_t:'分類管理',pg_cats_d:'用分類整理項目,方便訪客篩選',
pg_stats_t:'統計',pg_stats_d:'近 30 天的下載、複製趨勢和訪問來源',
pg_sec_t:'安全',pg_sec_d:'修改登入密碼,管理 IP 黑名單',
st_items:'項目總數',st_cats:'分類數量',st_dl_total:'累計下載',st_dl_today:'今日下載',st_cp_today:'今日複製',st_dead:'失效連結',
remind_title:'提醒事項',rm_dead:'連結失效:{0}',rm_unchecked:'有 {0} 個項目尚未檢測',rm_uncat:'有 {0} 個項目未分類',
rm_ok:'一切正常,沒有待處理事項',rm_check:'立即檢測',rm_view:'處理',rm_checked:'檢測於 {0}',rm_never:'從未檢測',
f_name:'名稱 *',f_url:'位址 * (http/https/rtmp...)',f_desc:'說明',no_cat:'(無分類)',sort:'排序',sort_tip:'排序,越小越靠前',
add:'新增',save:'儲存修改',cancel:'取消編輯',upload:'上傳檔案',uploading:'上傳中...',
search:'搜尋',all_cats:'全部分類',edit:'編輯',del:'刪除',status:'狀態',ok:'正常',unknown:'未檢測',
th_cat:'分類',th_sort:'排序',th_copies:'複製',th_ops:'操作',
b_del:'批量刪除',b_move:'移至分類',drag_tip:'拖曳列可調整順序(僅在未搜尋、未篩選時生效)',
check_now:'立即檢測失效連結',check_started:'已開始背景檢測,稍後重新整理查看結果',
confirm_del:'確定刪除該項目?',confirm_del_n:'確定刪除所選 {0} 項?',confirm_cat_del:'刪除分類後,其下項目會變為「無分類」,繼續?',
cat_name_ph:'分類名稱',cat_count:'項目數',prompt_name:'分類名稱',prompt_sort:'排序(越小越靠前)',
stats_trend:'近 30 天趨勢',downloads:'下載',copies:'複製',stats_sources:'訪問來源',stats_top:'熱門項目',no_data:'暫無資料',
source:'來源',times:'次數',
old_pw:'舊密碼',new_pw:'新密碼(至少 8 位)',change_pw:'修改密碼',pw_ok:'密碼已修改,其他裝置已登出',
bl_title:'IP 黑名單',ip_ph:'IP 位址',note_ph:'備註',bl_add:'加入黑名單',unblock:'解除',th_time:'時間',
site_settings:'網站設定',name_dl_sw:'允許點擊名稱下載(跳轉)',save_ok:'已儲存',
err_400:'參數錯誤(位址需以 http/https 等開頭,僅限 ASCII,不含空格)',err_401:'使用者名稱或密碼錯誤',err_401s:'登入已過期,請重新登入',
err_403:'存取被拒絕',err_404:'不存在',err_409:'已存在(位址、分類名重複或檢測進行中)',err_422:'舊密碼錯誤',
err_429:'嘗試次數過多,請 15 分鐘後再試',err_502:'物件儲存上傳失敗',err_x:'錯誤 {0}'
},
'en':{
site_title:'File Share',admin_title:'Admin',back_home:'Home',admin_link:'Admin',
pg_home_t:'File Share',pg_home_d:'Search, copy links, or download the files you need',
search_ph:'Search name or description...',th_req:'Type',th_name:'Name',th_addr:'URL (copyable)',th_desc:'Description',th_dl:'Downloads',
req:'Required',opt:'Optional',copy:'Copy',copied:'Copied',all:'All',none_found:'Nothing found',
prev:'Prev',next:'Next',page_info:'Page {0} / {1}, {2} total',dead:'Dead',
ui_native:'Native Dark',ui_saas:'SaaS Light',ui_tip:'Interface style',accent_tip:'Accent color',lang_tip:'Language',role_admin:'Admin',
pg_login_t:'Welcome back',pg_login_d:'Sign in to manage your file share site',
login:'Login',username:'Username',password:'Password',logout:'Logout',
nav_dashboard:'Dashboard',tab_items:'Items',tab_cats:'Categories',tab_stats:'Stats',tab_sec:'Security',
pg_dash_t:'Dashboard',pg_dash_d:'Site overview and things that need attention',
pg_items_t:'Items',pg_items_d:'Add, edit, upload, reorder and bulk-manage file entries',
pg_cats_t:'Categories',pg_cats_d:'Organize items so visitors can filter them',
pg_stats_t:'Stats',pg_stats_d:'Downloads, copies and referrers over the last 30 days',
pg_sec_t:'Security',pg_sec_d:'Change your password and manage the IP blacklist',
st_items:'Total items',st_cats:'Categories',st_dl_total:'Total downloads',st_dl_today:'Downloads today',st_cp_today:'Copies today',st_dead:'Dead links',
remind_title:'Reminders',rm_dead:'Dead link: {0}',rm_unchecked:'{0} items have not been checked yet',rm_uncat:'{0} items have no category',
rm_ok:'All good, nothing needs attention',rm_check:'Check now',rm_view:'Handle',rm_checked:'Checked {0}',rm_never:'Never checked',
f_name:'Name *',f_url:'URL * (http/https/rtmp...)',f_desc:'Description',no_cat:'(No category)',sort:'Sort',sort_tip:'Sort order, smaller first',
add:'Add',save:'Save',cancel:'Cancel',upload:'Upload file',uploading:'Uploading...',
search:'Search',all_cats:'All categories',edit:'Edit',del:'Delete',status:'Status',ok:'OK',unknown:'Unchecked',
th_cat:'Category',th_sort:'Sort',th_copies:'Copies',th_ops:'Actions',
b_del:'Delete selected',b_move:'Move selected to',drag_tip:'Drag rows to reorder (only without search/filter)',
check_now:'Check links now',check_started:'Check started in background, refresh later',
confirm_del:'Delete this item?',confirm_del_n:'Delete {0} selected items?',confirm_cat_del:'Items in this category will become uncategorized. Continue?',
cat_name_ph:'Category name',cat_count:'Items',prompt_name:'Category name',prompt_sort:'Sort (smaller first)',
stats_trend:'Last 30 days',downloads:'Downloads',copies:'Copies',stats_sources:'Referrers',stats_top:'Top items',no_data:'No data',
source:'Source',times:'Count',
old_pw:'Old password',new_pw:'New password (min 8)',change_pw:'Change password',pw_ok:'Password changed. Other devices were logged out',
bl_title:'IP blacklist',ip_ph:'IP address',note_ph:'Note',bl_add:'Block',unblock:'Remove',th_time:'Time',
site_settings:'Site settings',name_dl_sw:'Allow clicking name to download (redirect)',save_ok:'Saved',
err_400:'Invalid input (URL must use a supported scheme, ASCII only, no spaces)',err_401:'Wrong username or password',err_401s:'Session expired, please log in again',
err_403:'Access denied',err_404:'Not found',err_409:'Already exists (duplicate URL/name, or a check is running)',err_422:'Old password is wrong',
err_429:'Too many attempts, try again in 15 minutes',err_502:'Object storage upload failed',err_x:'Error {0}'
}};

function detectLang(){
  const l=(navigator.language||'en').toLowerCase();
  if(l.startsWith('zh'))return /tw|hk|mo|hant/.test(l)?'zh-TW':'zh-CN';
  return 'en';
}
let LANG=localStorage.getItem('lang')||detectLang();
if(!I18N[LANG])LANG='zh-CN';

function t(k,...a){
  let s=(I18N[LANG]&&I18N[LANG][k])||I18N['zh-CN'][k]||k;
  a.forEach((v,i)=>{s=s.replace('{'+i+'}',v)});
  return s;
}
function errMsg(status){
  const k='err_'+status;
  return I18N[LANG][k]?t(k):t('err_x',status);
}
function applyI18n(){
  document.documentElement.lang=LANG;
  document.querySelectorAll('[data-i18n]').forEach(e=>e.textContent=t(e.dataset.i18n));
  document.querySelectorAll('[data-i18n-ph]').forEach(e=>e.placeholder=t(e.dataset.i18nPh));
  document.querySelectorAll('[data-i18n-title]').forEach(e=>e.title=t(e.dataset.i18nTitle));
  if(document.body.dataset.title)document.title=t(document.body.dataset.title);
}

/* ========== 两套 UI + 主题色 ========== */
const ACCENTS={blue:['#3b82f6','#6366f1'],purple:['#8b5cf6','#d946ef'],green:['#10b981','#06b6d4'],orange:['#f59e0b','#ef4444'],pink:['#ec4899','#f97316']};
function setUI(v){document.documentElement.dataset.ui=v;localStorage.setItem('ui',v);}
function setAccent(v){
  document.documentElement.dataset.accent=v;localStorage.setItem('accent',v);
  document.querySelectorAll('.dot').forEach(d=>d.classList.toggle('on',d.dataset.a===v));
}
(function(){
  const ui=localStorage.getItem('ui')||(matchMedia('(prefers-color-scheme: dark)').matches?'native':'saas');
  document.documentElement.dataset.ui=(ui==='native'||ui==='saas')?ui:'native';
  const ac=localStorage.getItem('accent');
  document.documentElement.dataset.accent=ACCENTS[ac]?ac:'blue';
})();

/* ========== 右上角工具:界面风格 / 主题色 / 语言 ========== */
function mountTools(){
  const el=document.getElementById('tools');
  if(!el)return;
  el.innerHTML=
    '<select id="uiSel" data-i18n-title="ui_tip"><option value="native" data-i18n="ui_native"></option><option value="saas" data-i18n="ui_saas"></option></select>'+
    '<span class="accents" data-i18n-title="accent_tip">'+Object.keys(ACCENTS).map(k=>
      `<button class="dot" data-a="${k}" style="background:linear-gradient(135deg,${ACCENTS[k][0]},${ACCENTS[k][1]})"></button>`).join('')+'</span>'+
    '<select id="langSel" data-i18n-title="lang_tip"><option value="zh-CN">简体中文</option><option value="zh-TW">繁體中文</option><option value="en">English</option></select>';
  $('#uiSel').value=document.documentElement.dataset.ui;
  $('#uiSel').onchange=e=>setUI(e.target.value);
  el.querySelectorAll('.dot').forEach(d=>d.onclick=()=>setAccent(d.dataset.a));
  setAccent(document.documentElement.dataset.accent);
  $('#langSel').value=LANG;
  $('#langSel').onchange=e=>{
    LANG=e.target.value;localStorage.setItem('lang',LANG);
    applyI18n();
    if(window.onLangChange)window.onLangChange();
  };
}
document.addEventListener('DOMContentLoaded',()=>{mountTools();applyIcons();applyI18n();});
