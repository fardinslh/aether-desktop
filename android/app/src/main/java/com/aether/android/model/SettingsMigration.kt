package com.aether.android.model
import com.google.gson.Gson
import com.google.gson.JsonParser
object SettingsMigration {
 fun decode(json:String):AppSettings {
  val gson=Gson();val stored=JsonParser.parseString(json).asJsonObject
  val merged=gson.toJsonTree(AppSettings()).asJsonObject
  if(!stored.has("connectionMode"))merged.addProperty("connectionMode","manual")
  for((key,value) in stored.entrySet())merged.add(key,value)
  return gson.fromJson(merged,AppSettings::class.java)
 }
}
