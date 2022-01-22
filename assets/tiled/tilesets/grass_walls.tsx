<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.5" tiledversion="1.7.2" name="wang_tileset" tilewidth="32" tileheight="32" tilecount="18" columns="6">
 <image source="../textures/wang_tileset.png" width="192" height="96"/>
 <tile id="1">
  <objectgroup draworder="index" id="2">
   <object id="1" template="../templates/right_elbow.tx" x="32" y="32" rotation="180"/>
  </objectgroup>
 </tile>
 <tile id="2">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/edge.tx" x="0" y="16" rotation="270"/>
  </objectgroup>
 </tile>
 <tile id="3">
  <objectgroup draworder="index" id="2">
   <object id="1" template="../templates/left_elbow.tx" x="32" y="32" rotation="180"/>
  </objectgroup>
 </tile>
 <tile id="4">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/corner.tx" x="16" y="16"/>
  </objectgroup>
 </tile>
 <tile id="5">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/corner.tx" x="0" y="16"/>
  </objectgroup>
 </tile>
 <tile id="6">
  <objectgroup draworder="index" id="2">
   <object id="1" type="wall" x="0" y="0" width="32" height="32"/>
  </objectgroup>
 </tile>
 <tile id="7">
  <objectgroup draworder="index" id="2">
   <object id="1" template="../templates/edge.tx" x="0" y="0"/>
  </objectgroup>
 </tile>
 <tile id="9">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/edge.tx" x="16" y="0"/>
  </objectgroup>
 </tile>
 <tile id="10">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/corner.tx" x="16" y="0"/>
  </objectgroup>
 </tile>
 <tile id="11">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/corner.tx" x="0" y="0"/>
  </objectgroup>
 </tile>
 <tile id="13">
  <objectgroup draworder="index" id="2">
   <object id="1" template="../templates/left_elbow.tx" x="0" y="0"/>
  </objectgroup>
 </tile>
 <tile id="14">
  <objectgroup draworder="index" id="2">
   <object id="2" template="../templates/edge.tx" x="32" y="16" rotation="90"/>
  </objectgroup>
 </tile>
 <tile id="15">
  <objectgroup draworder="index" id="2">
   <object id="1" template="../templates/right_elbow.tx" x="0" y="0"/>
  </objectgroup>
 </tile>
 <tile id="16">
  <objectgroup draworder="index" id="2">
   <object id="4" template="../templates/corner.tx" x="0" y="16"/>
   <object id="5" template="../templates/corner.tx" x="16" y="0"/>
  </objectgroup>
 </tile>
 <tile id="17">
  <objectgroup draworder="index" id="2">
   <object id="3" template="../templates/corner.tx" x="0" y="0"/>
   <object id="4" template="../templates/corner.tx" x="16" y="16"/>
  </objectgroup>
 </tile>
 <wangsets>
  <wangset name="grass_walls" type="corner" tile="6">
   <wangcolor name="walls" color="#ff0000" tile="6" probability="1"/>
   <wangcolor name="grass" color="#00ff00" tile="8" probability="1"/>
   <wangtile tileid="1" wangid="0,1,0,2,0,1,0,1"/>
   <wangtile tileid="2" wangid="0,1,0,2,0,2,0,1"/>
   <wangtile tileid="3" wangid="0,1,0,1,0,2,0,1"/>
   <wangtile tileid="4" wangid="0,2,0,1,0,2,0,2"/>
   <wangtile tileid="5" wangid="0,2,0,2,0,1,0,2"/>
   <wangtile tileid="6" wangid="0,1,0,1,0,1,0,1"/>
   <wangtile tileid="7" wangid="0,2,0,2,0,1,0,1"/>
   <wangtile tileid="8" wangid="0,2,0,2,0,2,0,2"/>
   <wangtile tileid="9" wangid="0,1,0,1,0,2,0,2"/>
   <wangtile tileid="10" wangid="0,1,0,2,0,2,0,2"/>
   <wangtile tileid="11" wangid="0,2,0,2,0,2,0,1"/>
   <wangtile tileid="13" wangid="0,2,0,1,0,1,0,1"/>
   <wangtile tileid="14" wangid="0,2,0,1,0,1,0,2"/>
   <wangtile tileid="15" wangid="0,1,0,1,0,1,0,2"/>
   <wangtile tileid="16" wangid="0,1,0,2,0,1,0,2"/>
   <wangtile tileid="17" wangid="0,2,0,1,0,2,0,1"/>
  </wangset>
 </wangsets>
</tileset>
