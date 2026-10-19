// Loopback-only audit proxy for third-party benchmark clients. No automatic retry.
import {createServer} from "node:http";
import {createHash,randomUUID} from "node:crypto";
import {mkdirSync,appendFileSync,writeFileSync} from "node:fs";
const key=process.env.OPENROUTER_API_KEY;
if(!key) throw new Error("OPENROUTER_API_KEY is missing");
const port=Number(process.env.RECOLLECT_BENCH_PROXY_PORT??8799);
const directory=process.env.RECOLLECT_BENCH_PROXY_DIR??".cache/benchmark-proxy";
mkdirSync(directory,{recursive:true});
const allowed=new Set(["qwen/qwen3-embedding-8b","z-ai/glm-5.3-flash","z-ai/glm-4.7-flash","openai/gpt-4o-2024-08-06"]);
const account=await fetch("https://openrouter.ai/api/v1/key",{headers:{Authorization:`Bearer ${key}`},signal:AbortSignal.timeout(15000)});
if(!account.ok) throw new Error("Account budget check failed");
const initial=(await account.json()).data;
if(initial.usage>=8||initial.limit_remaining<2) throw new Error("Campaign ceiling reached");
let committed=initial.usage, reserved=0, calls=0, fatal=false;
const failed=new Set();
const judgeRecovery=process.env.RECOLLECT_BENCH_JUDGE_RECOVERY==='1';
const attempts=new Map();
const ledger=(row)=>appendFileSync(directory+"/requests.jsonl",JSON.stringify({...row,at:new Date().toISOString()})+"\n");
writeFileSync(directory+"/account.json",JSON.stringify({starting_usage:initial.usage,starting_remaining:initial.limit_remaining,ceiling_usd:8,judge_recovery:judgeRecovery,max_known_rate_limit_attempts:5},null,2));
const server=createServer(async(req,res)=>{
  const send=(code,body)=>{res.writeHead(code,{"Content-Type":"application/json"});res.end(JSON.stringify(body));};
  if(req.method!=="POST"||!['/v1/chat/completions','/v1/embeddings'].includes(req.url)) return send(404,{error:{message:"Unsupported benchmark endpoint"}});
  const id=randomUUID(); let reservation=0,digest;
  try{
    const chunks=[];let bytes=0;
    for await(const chunk of req){bytes+=chunk.length;if(bytes>1_000_000) throw new Error("Input limit");chunks.push(chunk);}
    const body=JSON.parse(Buffer.concat(chunks));
    // The official unmodified LongMemEval evaluator names the direct API model.
    if(body.model==='gpt-4o-2024-08-06') body.model='openai/gpt-4o-2024-08-06';
    if(body.model==='openai/z-ai/glm-5.3-flash') body.model='z-ai/glm-5.3-flash';
    if(!allowed.has(body.model)) throw new Error("Unapproved benchmark model");
    digest=createHash('sha256').update(JSON.stringify(body)).digest('hex');
    if(fatal||failed.has(digest)||calls>=3000) throw new Error("Run stopped; reconcile ledger before another attempt");
    const previous=attempts.get(digest);
    if(previous?.attempts>=5) throw new Error('Bounded judge recovery attempts exhausted');
    if(previous?.retry_after>Date.now()){
      res.setHeader('Retry-After','10');
      ledger({id,event:'rejected',reason:'Known rate-limit cooldown; no upstream dispatch'});
      return send(429,{error:{message:'Known rate-limit cooldown'}});
    }
    const embedding=req.url.endsWith('/embeddings');
    if(!embedding){
      const output=body.max_tokens??body.max_completion_tokens??4096;
      if(output>4096) throw new Error("Output limit");
      body.max_tokens=output;
      body.provider={require_parameters:true,allow_fallbacks:false};
      if(judgeRecovery&&body.model==='openai/gpt-4o-2024-08-06') body.provider.only=['openai'];
      if(body.model==='z-ai/glm-5.3-flash') body.reasoning={effort:'low'};
      if(body.model==='z-ai/glm-4.7-flash') body.reasoning={enabled:false};
      // Deliberately conservative reservation, above every selected tariff.
      reservation=bytes*2*5/1_000_000+output*20/1_000_000;
    }else{
      // Cognee's compatible engine configures 1024 dimensions but omits the
      // field on the wire. Transmit that declared benchmark configuration.
      body.dimensions ??= 1024;
      body.provider={allow_fallbacks:false};
      reservation=bytes*2*0.04/1_000_000;
    }
    if(committed+reserved+reservation>8) throw new Error("Campaign reservation exceeds USD 8");
    calls++; reserved+=reservation;
    attempts.set(digest,{attempts:(previous?.attempts??0)+1,retry_after:0});
    ledger({id,event:'reserved',model:body.model,input_bytes:bytes,reservation_usd:reservation,body_sha256:digest});
    const upstream=await fetch(`https://openrouter.ai/api/v1${req.url.slice(3)}`,{method:'POST',headers:{Authorization:`Bearer ${key}`,'Content-Type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(60000)});
    const raw=await upstream.text();
    if(raw.length>8_000_000) throw new Error("Response limit");
    const value=JSON.parse(raw);
    const actual=value.usage?.cost;
    if(!upstream.ok || !Number.isFinite(actual) || actual<0) {
      if(judgeRecovery&&body.model==='openai/gpt-4o-2024-08-06'&&upstream.status===429){
        attempts.get(digest).retry_after=Date.now()+10000;
        ledger({id,event:'failed',http_status:429,error_code:value.error?.code,reservation_usd:reservation});
        res.setHeader('Retry-After','10');
        return send(429,{error:{message:'Known judge rate limit; bounded authorized recovery'}});
      }
      fatal=true;failed.add(digest);
      ledger({id,event:'failed',http_status:upstream.status,error_code:value.error?.code,reservation_usd:reservation});
      return send(400,{error:{message:'Benchmark provider failed; audit ledger before any new run'}});
    }
    reserved-=reservation;committed+=actual;reservation=0;
    mkdirSync(directory+'/responses',{recursive:true});
    writeFileSync(directory+'/responses/'+id+'.json',JSON.stringify(value).replaceAll(key,'[REDACTED]')+'\n');
    const returnedMatches=value.model===body.model || (embedding && body.model==='qwen/qwen3-embedding-8b' && value.model==='Qwen/Qwen3-Embedding-8B');
    const finish=value.choices?.[0]?.finish_reason;
    const permittedFinish=finish==='stop'||(body.model==='openai/gpt-4o-2024-08-06'&&finish==='length');
    if(!returnedMatches || (!embedding && (value.choices?.length!==1 || !permittedFinish || typeof value.choices[0].message?.content!=='string'))){
      fatal=true;failed.add(digest);
      ledger({id,event:'invalid_response',requested_model:body.model,returned_model:value.model,actual_cost_usd:actual,finish_reason:value.choices?.[0]?.finish_reason});
      return send(400,{error:{message:'Benchmark response identity or completion invalid; retain paid receipt'}});
    }
    ledger({id,event:'completed',model:value.model,usage:value.usage,actual_cost_usd:actual,finish_reason:value.choices?.[0]?.finish_reason});
    send(200,value);
  }catch(error){
    ledger({id,event:'rejected',reason:String(error.message).replaceAll(key,'[REDACTED]').slice(0,160)});
    if(reservation){fatal=true;failed.add(digest);ledger({id,event:'uncertain',reservation_usd:reservation});}
    send(400,{error:{message:String(error.message).replaceAll(key,'[REDACTED]').slice(0,160)}});
  }
});
server.listen(port,'127.0.0.1',()=>console.log(JSON.stringify({listening:`http://127.0.0.1:${port}/v1`,directory,starting_usage:initial.usage})));
process.on('SIGTERM',()=>server.close(()=>process.exit(0)));
