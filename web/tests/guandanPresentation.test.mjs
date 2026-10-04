import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { describePlays, beats, isWild, advanceGuandanPresentation } from "../src/guandanPresentation.ts";
const combo=(cards,level,kind)=>describePlays(cards,level).find(c=>c.kind===kind);
test("two-deck identity, level singles and joker restrictions",()=>{
  assert.equal(isWild(49,2),true);assert.equal(isWild(103,2),true);assert.equal(isWild(51,2),false);
  assert.equal(combo([20],8,"single").high,15);assert.equal(combo([52,106],2,"pair").high,16);
  assert.equal(describePlays([52,49],2).length,0);
  for(const cards of [[],[0,0],[108],[-1],[0.5],[52,53],[52,106,53]])assert.deepEqual(describePlays(cards,2),[]);
});
test("wildcard sequences retain multiple legal declarations and strict bomb ordering",()=>{
  const wildFlush=describePlays([0,4,8,12,49],2);
  assert.ok(wildFlush.some(c=>c.kind==="straight_flush"));assert.ok(wildFlush.some(c=>c.kind==="straight"));
  const flush=combo([0,4,8,12,16],2,"straight_flush");
  assert.equal(beats(flush,combo([0,1,2,3,54],2,"bomb")),true);
  assert.equal(beats(combo([0,1,2,3,54,55],2,"bomb"),flush),true);
  assert.equal(beats(combo([52,53,106,107],2,"rocket"),combo([0,1,2,3,54,55,56,57,49,103],2,"bomb")),true);
  assert.equal(combo([44,48,0,4,8],7,"straight").high,5);
  assert.equal(combo([28,32,36,40,44],10,"straight").high,14);
  assert.equal(describePlays([40,44,48,0,4],7).length,0);
});
function room(){return {id:"gd",round:1,selfSeat:0,gameId:"guandan",game:{phase:"playing",turn:0,legal_actions:[{type:"play"}],events:[{seq:1,kind:"deal",message:"发牌",cards:[],seat:null,voice:"gd/deal",combination:null}]}};}
test("event-specific declaration and voice survive snapshots; hydrate and resume remain silent",()=>{
  const r=room();let result=advanceGuandanPresentation(null,r,true);assert.deepEqual(result.feedback,[]);
  r.game.turn=1;r.game.events.push({seq:2,kind:"play",seat:0,cards:[0,4,8,12,49],message:"顺子",voice:"gd/straight",combination:combo([0,4,8,12,49],2,"straight")});
  const fresh=advanceGuandanPresentation(result.cursor,r,true);assert.equal(fresh.feedback[0].kind,"straight");assert.equal(fresh.feedback[0].voice,"gd/straight");
  assert.deepEqual(advanceGuandanPresentation(fresh.cursor,r,true).feedback,[]);
  const suspended=advanceGuandanPresentation(fresh.cursor,r,false);r.game.events.push({...r.game.events[1],seq:3});assert.deepEqual(advanceGuandanPresentation(suspended.cursor,r,true).feedback,[]);
});
test("live new rounds announce deal and return prompt, identity changes stay silent",()=>{
  const r=room(),old=advanceGuandanPresentation(null,r,true).cursor;r.round=2;r.game.phase="returning";
  const next=advanceGuandanPresentation(old,r,true);assert.equal(next.feedback[0].kind,"deal");assert.equal(next.feedback.at(-1).voice,"gd/your-return");
  r.selfSeat=2;assert.deepEqual(advanceGuandanPresentation(next.cursor,r,true).feedback,[]);
});
test("every declared offline voice is bundled and encoded",()=>{
  const base=new URL("../public/audio/voice/gd/",import.meta.url);const manifest=JSON.parse(readFileSync(new URL("manifest.json",base),"utf8").replace(/^\uFEFF/,""));
  assert.equal(Object.keys(manifest).length,64);
  for(const key of Object.keys(manifest)){const data=readFileSync(new URL(`${key}.mp3`,base));assert.ok(data.length>1000,key);}
  assert.ok(existsSync(new URL("../public/audio/bgm.mp3",import.meta.url)));
});
