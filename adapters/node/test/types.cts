import api = require('@humanymous/node');
api.express({installation:'test',tenant:'t',origin:'o',revision:'r',routes:['/'],socketPath:'/run/decision.sock',credential:'a'.repeat(32)});
