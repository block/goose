// Top Selling Items — Recharts bar chart following Square design guidelines.
//
// This is a React chart artifact. The desktop app provides React, ReactDOM,
// and Recharts as globals. Post via:
//   --mime-type application/vnd.managerbot.web-preview
//   --body '{"react": "<contents>", "title": "Top Selling Items", "height": 520}'

const { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } = Recharts;
const e = React.createElement;

// --- Square design tokens ---
const INK      = '#101010';  // text-10
const MUTED    = '#666666';  // text-20
const SUBTLE   = '#959595';  // text-30
const GRID     = '#f0f0f0';  // divider-20
const BG       = '#F7F7F7';  // fill-50
const ACCENT   = '#006AFF';  // emphasis fill
const LIGHT    = '#DCE9FF';

// --- Data (would come from /tmp/chart-data.json in real use) ---
const data = [
  { name: 'Chicken Shawarma Plate', qty: 312 },
  { name: 'Lamb Kebab Wrap', qty: 287 },
  { name: 'Falafel Bowl', qty: 245 },
  { name: 'Hummus & Pita', qty: 198 },
  { name: 'Turkish Coffee', qty: 176 },
  { name: 'Baklava', qty: 134 },
  { name: 'Greek Salad', qty: 98 },
  { name: 'Lentil Soup', qty: 67 },
];

const maxQty = Math.max(...data.map(d => d.qty));

const styles = {
  page: {
    maxWidth: 720, margin: '0 auto', padding: '32px 24px',
    fontFamily: "'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif",
    color: INK, background: BG,
  },
  card: {
    background: '#fff', borderRadius: 12, border: `1px solid ${GRID}`, padding: '24px 28px',
  },
  title: { fontSize: 22, fontWeight: 700, marginBottom: 4 },
  subtitle: { fontSize: 11, color: MUTED, marginBottom: 24 },
  footer: { fontSize: 10, color: SUBTLE, marginTop: 16 },
};

const CustomTooltip = ({ active, payload }) => {
  if (!active || !payload?.length) return null;
  const d = payload[0].payload;
  return e('div', {
    style: {
      background: INK, color: '#fff', padding: '8px 12px', borderRadius: 8,
      fontSize: 13, fontFamily: styles.page.fontFamily,
    },
  }, `${d.name}: ${d.qty.toLocaleString()} units`);
};

const App = () => e('div', { style: styles.page },
  e('div', { style: styles.card },
    e('div', { style: styles.title }, 'Top selling items'),
    e('div', { style: styles.subtitle }, 'Units sold · Jan 1 – Dec 31, 2025'),

    e(ResponsiveContainer, { width: '100%', height: 340 },
      e(BarChart, { data, layout: 'vertical', margin: { left: 20, right: 40 } },
        e(CartesianGrid, { strokeDasharray: '3 3', stroke: GRID, horizontal: false }),
        e(XAxis, {
          type: 'number', tick: { fontSize: 11, fill: MUTED },
          axisLine: false, tickLine: false,
          label: { value: 'Units sold', position: 'insideBottom', offset: -4, fontSize: 11, fill: MUTED },
        }),
        e(YAxis, {
          type: 'category', dataKey: 'name', width: 140,
          tick: { fontSize: 11, fill: INK, fontWeight: 500 },
          axisLine: false, tickLine: false,
        }),
        e(Tooltip, { content: e(CustomTooltip), cursor: { fill: 'rgba(0,0,0,0.03)' } }),
        e(Bar, { dataKey: 'qty', radius: [0, 6, 6, 0], barSize: 28 },
          data.map((d, i) =>
            e(Cell, { key: i, fill: d.qty === maxQty ? ACCENT : LIGHT })
          ),
        ),
      ),
    ),

    e('div', { style: styles.footer },
      `${data[0].name} led the year with ${data[0].qty.toLocaleString()} units sold.`
    ),
  ),
);

ReactDOM.render(e(App), document.getElementById('root'));
