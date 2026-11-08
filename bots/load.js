// Mineflayer load: offline-mode bots connect through Velocity, hit the nearest player, and stay idle (NOMOVE=1) or wander.
// Usage: node load.js <bots> <seconds> [port=15590] [prefix=pb]
// NOMOVE=1: movement packets are dropped so bots stand still. CHAT=1: each bot chats every 3 s.
// Prints one JSON summary line at the end.
const mineflayer = require('mineflayer')
const [n = 10, dur = 180, port = 15590, prefix = 'pb'] = process.argv.slice(2).map((v, i) => (i === 3 ? v : Number(v)))
const ev = { spawned: 0, kicked: 0, errors: 0, ended: 0 }
const kicks = []
const bots = []
for (let i = 0; i < n; i++) {
  setTimeout(() => {
    const bot = mineflayer.createBot({ host: '127.0.0.1', port, version: '26.1', username: `${prefix}${i}`, auth: 'offline', hideErrors: true, checkTimeoutInterval: 60000, physicsEnabled: !process.env.NOMOVE })
    bots.push(bot)
    if (process.env.NOMOVE) {
      const w = bot._client.write.bind(bot._client)
      bot._client.write = (name, p) => (/^(position|position_look|look|flying)$/.test(name) ? undefined : w(name, p))
    }
    bot.once('spawn', () => {
      ev.spawned++
      if (process.env.CHAT) setInterval(() => bot.chat(`hello from ${bot.username}`), 3000)
      setInterval(() => { // hit the nearest player every 1.5 s (any distance with NOMOVE)
        const t = bot.nearestEntity(e => e.type === 'player' && e.username !== bot.username)
        if (t && (process.env.NOMOVE || bot.entity.position.distanceTo(t.position) < 4)) { bot.lookAt(t.position.offset(0, 1.6, 0), true); bot.attack(t) }
      }, 1500)
      if (!process.env.NOMOVE) {
        setInterval(() => {
          if (!bot.entity) return
          bot.setControlState('forward', Math.random() < 0.8)
          bot.setControlState('jump', Math.random() < 0.3)
          bot.look(Math.random() * Math.PI * 2, (Math.random() - 0.5) * 0.6, true)
        }, 1000)
      }
    })
    bot.on('kicked', r => { ev.kicked++; kicks.push(String(JSON.stringify(r)).slice(0, 200)) })
    bot.on('error', e => { ev.errors++; kicks.push('err:' + e.message.slice(0, 150)) })
    bot.on('end', () => { ev.ended++ })
  }, i * 300)
}
setTimeout(() => {
  console.log(JSON.stringify({ n, dur, ...ev, stillConnected: bots.filter(b => b.entity).length, kicks: [...new Set(kicks)].slice(0, 5) }))
  process.exit(0)
}, dur * 1000)
